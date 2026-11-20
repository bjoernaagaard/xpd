Use `xpd hook` as the hook command of a coding agent. The client sends each event as JSON on
stdin, and xpd reads the event's name from `hook_event_name`. The
[Claude Code guide](../../guides/claude-code/) shows the setup.

| Event | What xpd does |
| --- | --- |
| `SessionStart` | Records the commit that is checked out, and prints a brief of the project for the agent |
| `UserPromptSubmit` | Prints the playbook that answers the prompt, or the open issues of the project when the prompt asks what to improve |
| `PostToolUse` | Lints the files that the edit changed, so the agent sees their issues |
| `Stop` | Lints the Python files that the session changed, and holds the turn while one has an error or a warning |

The brief and the prompt answer are plain text on stdout, which Claude Code and Codex both add to
what the agent reads. Each states what xpd found, and neither gives the agent an order. xpd prints
them only in a project with a Python file, and each is at most 2,000 characters long.

## When a session starts

On `SessionStart`, xpd first records the commit that is checked out, which the `Stop` check
compares the files with. It then reads the Python files of the project and prints the brief below.
This one is from the xpd repository.

```text
xpd read 153 Python files of this project.
Rule packs that apply: python (153 files), numpy (68 files), pandas (66 files), sklearn (31 files), plotly (30 files), arch (11 files), linearmodels (7 files), mapie (7 files), pymoo (7 files), category_encoders (6 files), sklego (6 files), scipy (5 files), hamilton (1 file).
The style setting is strict, so a rule of the house style is a warning, and the Stop check holds a turn that leaves a warning.
Pipelines of steps that `xpd graph` reads: 7 in 6 files: samples/graph/chain.py (2), samples/graph/steps.py (1), samples/graph/steps_wrong_order.py (1), and 3 more.
Hamilton nodes that `xpd graph` reads: 19 in 1 file: samples/hamilton/mutation.py (19).
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

xpd reads at most 1,000 Python files and looks at no more than 20,000 files and folders, so the
brief stays fast in a large project. The brief says so when it stopped at 1,000 files. Folders
respect `.gitignore`, as in `xpd check`. On the 153 files of the xpd repository, the release build
printed the brief in 0.5 seconds. After a compaction, the client sends `SessionStart` again and
xpd prints the brief again.

## On each prompt

On `UserPromptSubmit`, xpd searches the prompt as [`xpd find`](../find/) does, and it decides what
to print as follows.

```text
on(UserPromptSubmit)
  if the project has no Python file
    print nothing
  if the first recipe that find gives for the prompt is a playbook, and its keyword score is 24 or more
    print the playbook       # its name, its commands in order, its rules and its name for xpd api
  if the prompt asks what to make or improve
    print the open issues    # counted by pack and severity, with the five rules that have the most
  otherwise
    print nothing
```

A [playbook](../find/#playbooks) is one of xpd's own recipes, which holds the house style for a
kind of task. A prompt asks what to make or improve when it holds one of the phrases "what should
I", "what should we", "what can I", "what could I", "what to make", "what to build", "what to
improve", "what to do next" or "what next", or one of the words "improve", "optimize" or "better".

| Prompt | What xpd prints |
| --- | --- |
| "improve my charts" | The playbook `xpd/recipe/plotly-chart` and the open issues |
| "build a robust pandas pipeline for feature engineering" | The playbook `xpd/recipe/pandas-pipeline` |
| "make this linear" | The playbook `xpd/recipe/linear-script` |
| "what should I make?" | The open issues |
| "rename this variable" | Nothing |
| "why does the merge duplicate rows?" | Nothing |

The score of 24 is the keyword score that `xpd find` ranks by. Ten loose asks, among them every
ask that `indexes/questions.json` expects a playbook for, scored 28 and above for their playbook.
Thirty prompts about other work got no playbook as their first recipe, and the best playbook of
each scored 18 and below. The hook searches by meaning only when the search model of `xpd find` is
already on the machine, and it never downloads the model. The score comes from the keywords alone,
so the same prompts get a playbook with and without the model.

The open issues are the ones that `xpd check` reports for the first 1,000 Python files of the
project, under the style setting of the project.

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
and every untracked Python file, when the file was written after the session began. A file that
was already changed or untracked before the session is not the session's own, so xpd leaves it
out. This also covers a file that the agent changed through the shell
and a file that it committed. A file that only a pull, a merge, a rebase or a checkout changed is
not the session's own either, so xpd leaves it out. xpd reads this from git's reflog, which says
how each commit was checked out. In a project without git, xpd lints the Python files that the
`PostToolUse` events of the session named.

While a file has an error or a warning, xpd prints `{"decision": "block", "reason": ...}` on
stdout, with one line for each issue. Claude Code and Codex both read this answer and keep the
agent working. Issues of the severity `info` are listed after the others, and they do not hold the
turn. With [`style = "strict"`](../check/#house-style) in the settings, the rules of the house
style are warnings, so they hold the turn too.

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

A `UserPromptSubmit` event in a project with one script that draws four plotly figures gives this
answer.

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
xpd checked 1 Python file of this project under the strict style and found 12 open issues.
plotly: 11 warnings
pandas: 1 warning
The rules with the most issues: pandas/pivot-table-default-mean (1), plotly/all-caps-title (1), plotly/bar-axis-without-zero (1), plotly/default-template (1), plotly/explicit-grid (1).
`xpd check . --format json` lists each issue with its file and line, and `xpd rules --rule <pack>/<rule>` explains a rule.
```

A `Stop` event in a project where the session added `loop.py` gives this answer.

```console
$ echo '{"session_id":"s1","cwd":"/tmp/project","hook_event_name":"Stop","stop_hook_active":false}' | xpd hook
{"decision":"block","reason":"xpd holds this turn, because the Python files that this session changed have errors or warnings. Fix each line below. When the code is right as it is, write `# ast-grep-ignore: <pack>/<rule>` on the line above it, with the reason after the rule.\nloop.py:4:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df[\"a\"] * df[\"w\"]. When each row has to become a Python object, loop over itertuples() or to_dict(\"records\")."}
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
