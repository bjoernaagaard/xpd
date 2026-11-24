Use `xpd hook` as the hook command of a coding agent. The client sends each event as JSON on
stdin, and xpd reads the event's name from `hook_event_name`. The
[Claude Code guide](../../guides/claude-code/) shows the setup.

| Event | What xpd does |
| --- | --- |
| `SessionStart` | Records the commit that is checked out and the project's drift from its profile, and prints a brief of the project for the agent |
| `UserPromptSubmit` | Prints the playbook that answers the prompt, or the open issues of the project when the prompt asks what to improve |
| `PostToolUse` | Lints the files that the edit changed, so the agent sees their issues |
| `Stop` | Lints the Python files that the session changed, and holds the turn while one has an error or a warning, or while the session moved the project away from its profile |

The brief and the prompt answer are plain text on stdout, which Claude Code and Codex both add to
what the agent reads. Each states what xpd found, and neither gives the agent an order. xpd prints
them only in a project with a Python file, and keeps each response short.

## When a session starts

On `SessionStart`, xpd records the checked-out commit and the Git blob IDs of Python files
with uncommitted changes, including untracked files. In a project whose `pyproject.toml` has a
`[tool.xpd.profile]` table, it also records each difference between the project and the profile
of [`xpd doctor`](../doctor/). It then reads the Python files of the project and prints a brief.

```console
$ mkdir -p brief && cp samples/graph/steps.py brief/
$ echo '{"session_id":"s0","cwd":"/tmp/project/brief","hook_event_name":"SessionStart"}' | xpd hook
xpd read 1 Python file of this project.
Rule packs that apply: pandas (1 file), python (1 file).
The style setting is default, so a rule of the house style is info, which holds no turn.
Pipelines of steps that `xpd graph` reads: 1 in 1 file: steps.py (1).
Hamilton nodes that `xpd graph` reads: none.
Three xpd commands fit between the edits of a session.
- `xpd find "<task>"` ranks library symbols, rules and playbooks for a task, and it comes before the choice of a library or a method.
- `xpd graph <file>` prints the dataflow of a file, and it comes after a step or a node is added, moved or renamed.
- `xpd equiv old.py new.py` compares the result of two scripts, and a refactor of data logic is done when it prints {"equivalent": true}.
```

| Line of the brief | Where xpd reads it |
| --- | --- |
| Rule packs that apply | The imports of each Python file, as [`xpd check`](../check/) reads them |
| The style setting | `[tool.xpd]` in `pyproject.toml`, or the settings file of the user, as [House style](../check/#house-style) describes |
| Pipelines of steps | What [`xpd graph`](../graph/) finds in each file |
| Hamilton nodes | What `xpd graph` finds in each file that imports Hamilton |

xpd caps how many Python files and directory entries it scans so the brief stays fast in a large
project, and notes in the brief when it stops at the cap. Folders respect `.gitignore`, as in
`xpd check`. After a compaction, the client sends `SessionStart` again and xpd prints the brief
again.

## On each prompt

On `UserPromptSubmit`, xpd searches the prompt as [`xpd find`](../find/) does, and it decides what
to print as follows.

```text
on(UserPromptSubmit)
  if the project has no Python file
    print nothing
  if the first recipe that find gives for the prompt is a playbook and clears the keyword threshold
    print the playbook       # its name, its commands in order, its rules and its name for xpd api
  if the prompt asks what to make or improve
    print the open issues    # counted by pack and severity, with the five rules that have the most
  otherwise
    print nothing
```

A [playbook](../find/#playbooks) is one of xpd's own recipes, which holds the house style for a
kind of task. A prompt asks what to make or improve when it asks what to do next or asks to
improve, optimize or make the code better.

| Prompt | What xpd prints |
| --- | --- |
| "improve my charts" | The playbook `xpd/recipe/plotly-chart` and the open issues |
| "build a robust pandas pipeline for feature engineering" | The playbook `xpd/recipe/pandas-pipeline` |
| "make this linear" | The playbook `xpd/recipe/linear-script` |
| "what should I make?" | The open issues |
| "rename this variable" | Nothing |
| "why does the merge duplicate rows?" | Nothing |

The hook searches by meaning only when the search model of `xpd find` is already on the machine,
and it never downloads the model. The playbook threshold comes from the keywords alone, so the
same prompts get a playbook with and without the model.

The open issues are the ones that `xpd check` reports for the project's scanned Python files, under
the style setting of the project.

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
and every untracked Python file, subject to two ownership checks. First, xpd leaves out a file
whose contents match the uncommitted contents recorded when the session began. Shell edits and local
commits count even when a copied file retains an earlier modification time. Second, xpd leaves
out a file that only a pull, merge, rebase or checkout changed. Git's reflog records how each
commit was checked out. In a project without git, xpd lints the Python files that the
`PostToolUse` events of the session named.

While a file has an error or a warning, xpd prints `{"decision": "block", "reason": ...}` on
stdout, with one line for each issue. Claude Code and Codex both read this answer and keep the
agent working. Issues of the severity `info` are listed after the others, and they do not hold the
turn. With [`style = "strict"`](../check/#house-style) in the settings, the rules of the house
style are warnings, so they hold the turn too.

The agent ends the turn in one of two ways. It fixes the code, or it writes
`# ast-grep-ignore: pandas/row-loop` on the line above with the reason after the rule name.

In a project that records the profile, xpd compares the project with the profile again. It reads
files and runs `uv lock --check --offline` for each lock file, which reads uv's cache and never the
network. Each difference is named by its file, setting and value, without a line number, so an
edit elsewhere in `pyproject.toml` does not make an old difference look new. A difference that was not there when the session began holds the turn, with one line
for each, such as `pyproject.toml tool.ruff.lint.extend-select: ruff PTH118 os-path-join is off,
and the profile wants on`, or `uv.lock: the lock is out of date. xpd doctor --apply runs uv
lock.` after the session added a dependency without locking it. So does a `[tool.xpd.profile]`
table that the session removed. The agent undoes the change, runs `xpd doctor --apply`, or
records an exception with its reason, as the doctor page describes. A difference that was there
when the session began, or that xpd could not compare, reaches you as a notice and does not hold
the turn. So does a lock file that uv could not check, such as when uv is missing. xpd records the profile revision with the opening list, and
when an upgrade of xpd changes the revision during the session, every difference is a notice. The Stop check never writes the plan, installs a tool or refreshes a
lock.

xpd holds a turn once for the same list of issues. When the client sends `stop_hook_active` as
true and the errors and warnings are the ones that xpd held the turn on last time, xpd lets the
turn end. It then prints the list as `systemMessage`, which the client shows to you and not to the
agent. The `info` issues of a turn that ends reach you the same way.

xpd keeps one small file for each session in `sessions` in its cache folder, which is
`$XDG_CACHE_HOME/xpd` or `xpd` in the cache folder of your platform. `SessionStart` deletes stale
session files.

## Examples

```console
$ echo '{"hook_event_name":"PostToolUse","tool_name":"Edit","tool_input":{"file_path":"samples/bad.py"}}' | xpd hook 2>&1 | head -2
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
```

A `UserPromptSubmit` event whose prompt fits a playbook gives this answer, in a copy of the
repository's `samples/` folder. The `...` stands for the lines after the playbook, which count the
open issues of the project.

```console
$ echo '{"cwd":"/tmp/project","hook_event_name":"UserPromptSubmit","prompt":"improve my charts"}' | xpd hook
xpd has a playbook for this prompt: xpd/recipe/plotly-chart, "A plotly chart that passes the plotly pack".
`xpd api xpd/recipe/plotly-chart` prints its code in the house style.
Its commands, in order:
1. `xpd check <file> --format json` reports each form of the figure that the plotly pack argues against.
2. `xpd rules --rule plotly --format json` gives the documentation of each plotly rule, with the page of the book and the plotly code to write.
3. `xpd find "<chart form>" --kind rule` names the rule for a form, such as "small multiples" or "pie chart".
4. `xpd check <file> --format json` after the change, and the chart is done when it reports 0 issues.
The rules that its code satisfies: plotly/default-template, plotly/legend-instead-of-labels, plotly/axis-title-column-name, plotly/bar-axis-without-zero, plotly/area-axis-without-zero, plotly/explicit-grid, plotly/boxed-axis-frame, plotly/redundant-color, plotly/rainbow-colorscale, plotly/pie-chart, plotly/rotated-tick-labels, plotly/all-caps-title.
...
```

A `Stop` event after the session edited `samples/bad.py` gives this answer.

```console
$ echo '{"session_id":"s1","cwd":"/tmp/project","hook_event_name":"PostToolUse","tool_name":"Edit","tool_input":{"file_path":"samples/bad.py"}}' | xpd hook 2> /dev/null
$ echo '{"session_id":"s1","cwd":"/tmp/project","hook_event_name":"Stop","stop_hook_active":false}' | xpd hook
{"decision":"block","reason":"xpd holds this turn, because the Python files that this session changed have errors or warnings. Fix each line below. When the code is right as it is, write `# ast-grep-ignore: <pack>/<rule>` on the line above it, with the reason after the rule.\nsamples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df[\"a\"] * df[\"w\"]. When each row has to become a Python object, loop over itertuples() or to_dict(\"records\").\nsamples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, \"a\"] instead.\nsamples/bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.\nsamples/bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.\nThe lines below are advice, and they do not hold the turn.\nsamples/bad.py:7:1: info [pandas/no-inplace] inplace=True gives no speedup and blocks chaining. Reassign instead."}
```

## Exit status

Claude Code shows stderr to the agent only when a hook exits 2, so the hook uses that code for
the issues of an edit. A `Stop` answer is JSON on stdout, which both clients read only when the
hook exits 0. The brief and the prompt answer are plain text on stdout, which both clients also
read only when the hook exits 0.

| Exit code | Meaning |
| --- | --- |
| 0 | No issue, or an Agent Plugins client got the issues on stdout, or the event was `SessionStart`, `UserPromptSubmit` or `Stop` |
| 1 | xpd could not run, or stdin was not hook JSON |
| 2 | A Claude Code hook found issues after an edit, which it printed on stderr |
