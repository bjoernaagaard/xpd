Use `xpd hook` as a `PostToolUse` hook of a coding agent, so the agent sees the issues of each
file it edits. It reads the hook event JSON on stdin and lints the `.py` files that the event
changed. In a project with `sgconfig.yml`, it also lints the files that the project's rules are
for. When a file holds a pipeline of steps, it also reports the errors and warnings of
[`xpd graph`](../graph/). The [Claude Code guide](https://bjoernaagaard.github.io/xpd/claude-code.html)
shows the setup.

| Client | How the hook answers |
| --- | --- |
| A client of the Agent Plugins 1.1.0 draft | Prints the issues as `additional_context` in JSON on stdout |
| A hook in the Claude Code settings | Prints the issues on stderr |

## Examples

```console
$ echo '{"hook_event_name":"PostToolUse","tool_name":"Edit","tool_input":{"file_path":"samples/bad.py"}}' | xpd hook 2>&1 | head -2
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
```

## Exit status

Claude Code shows stderr to the agent only when a hook exits 2, so the hook uses that code for
issues.

| Exit code | Meaning |
| --- | --- |
| 0 | No issue, or an Agent Plugins client got the issues on stdout |
| 1 | xpd could not run, or stdin was not hook JSON |
| 2 | A Claude Code hook found issues, which it printed on stderr |
