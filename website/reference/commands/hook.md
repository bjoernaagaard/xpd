Use `xpd hook` as the hook command of a coding agent. The client sends each event as JSON on
stdin, and xpd reads the event's name from `hook_event_name`. The
[Claude Code guide](../../guides/claude-code/) shows the setup.

| Event | What xpd does |
| --- | --- |
| `SessionStart` | Records the commit that is checked out, and prints nothing |
| `PostToolUse` | Lints the files that the edit changed, so the agent sees their issues |
| `Stop` | Lints the Python files that the session changed, and holds the turn while one has an error or a warning |

## After an edit

On `PostToolUse`, xpd lints the `.py` files that the event changed. In a project with
`sgconfig.yml`, it also lints the files that the project's rules are for. When a file holds a
pipeline of steps, it also reports the errors and warnings of [`xpd graph`](../graph/).

| Client | How the hook answers |
| --- | --- |
| A client of the Agent Plugins 1.1.0 draft | Prints the issues as `additional_context` in JSON on stdout |
| A hook in the Claude Code settings | Prints the issues on stderr |

## When the agent wants to stop

On `Stop`, xpd lints every Python file that differs from the commit that `SessionStart` recorded,
and every untracked Python file. This also covers a file that the agent changed through the shell
and a file that it committed. In a project without git, xpd lints the Python files that the
`PostToolUse` events of the session named.

While a file has an error or a warning, xpd prints `{"decision": "block", "reason": ...}` on
stdout, with one line for each issue. Claude Code and Codex both read this answer and keep the
agent working. Issues of the severity `info` are listed after the others, and they do not hold the
turn.

The agent ends the turn in one of two ways. It fixes the code, or it writes
`# ast-grep-ignore: pandas/row-loop` on the line above with the reason after the rule name.

xpd holds a turn once for the same list of issues. When the client sends `stop_hook_active` as
true and the errors and warnings are the ones that xpd held the turn on last time, xpd lets the
turn end. It then prints the list as `systemMessage`, which the client shows to you and not to the
agent. The `info` issues of a turn that ends reach you the same way.

xpd keeps one small file for each session in `sessions` in its cache folder, which is
`$XDG_CACHE_HOME/xpd` or `xpd` in the cache folder of your platform. `SessionStart` deletes the
session files that are older than seven days.

## Examples

```console
$ echo '{"hook_event_name":"PostToolUse","tool_name":"Edit","tool_input":{"file_path":"samples/bad.py"}}' | xpd hook 2>&1 | head -2
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
```

A `Stop` event in a project where the session added `loop.py` gives this answer.

```console
$ echo '{"session_id":"s1","cwd":"/tmp/project","hook_event_name":"Stop","stop_hook_active":false}' | xpd hook
{"decision":"block","reason":"xpd holds this turn, because the Python files that this session changed have errors or warnings. Fix each line below. When the code is right as it is, write `# ast-grep-ignore: <pack>/<rule>` on the line above it, with the reason after the rule.\nloop.py:4:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df[\"a\"] * df[\"w\"]. When each row has to become a Python object, loop over itertuples() or to_dict(\"records\")."}
```

## Exit status

Claude Code shows stderr to the agent only when a hook exits 2, so the hook uses that code for
the issues of an edit. A `Stop` answer is JSON on stdout, which both clients read only when the
hook exits 0.

| Exit code | Meaning |
| --- | --- |
| 0 | No issue, or an Agent Plugins client got the issues on stdout, or the event was `SessionStart` or `Stop` |
| 1 | xpd could not run, or stdin was not hook JSON |
| 2 | A Claude Code hook found issues after an edit, which it printed on stderr |
