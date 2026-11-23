# xpd

Run `uvx xpd check .` to find the lines of your Python code that are slow or give a wrong result,
and add `--fix` to let xpd rewrite the ones it can. xpd checks code that uses pandas, Polars, NumPy,
SciPy, scikit-learn, scikit-lego, MAPIE, Category Encoders, linearmodels, arch, skfolio, pymoo, plotly
or Apache Hamilton. It reads the files and starts no Python, so you or a coding agent can run it
after every edit.

The documentation is at <https://bjoernaagaard.github.io/xpd/>, with a page for every rule.

## Install

xpd requires Python 3.13 or newer. Install it with [uv](https://docs.astral.sh/uv/).

```sh
uv tool install --python '>=3.13' xpd
```

You can also run xpd once without installing it, with `uvx xpd` in place of `xpd`.

## Check a file and fix it

Save the following code as `bad.py` in an empty folder. xpd reads the code and doesn't run it, so
you don't need pandas.

```python title="bad.py"
import pandas as pd

df = pd.DataFrame({"a": [1, 2], "k": [1, 2]})
other = df.copy()
for i, row in df.iterrows():
    df["a"][i] = row["a"] * 2
df.dropna(inplace=True)
m = pd.merge(df, other, on="k")
out = pd.DataFrame()
for f in ["x.csv"]:
    out = pd.concat([out, pd.read_csv(f)])
```

Run xpd on the file. On a terminal `--format text` is the default, and elsewhere xpd prints JSON.
It prints one line for each issue, with the place, the severity, the rule and
what to write instead, and it exits with 1 because one issue is an error.

```console
$ xpd check bad.py --format text
bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
bad.py:7:1: info [pandas/no-inplace] inplace=True gives no speedup and blocks chaining. Reassign instead.
bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: 5 issues, 2 more with --fix --unsafe-fixes
```

Fix every `error`, and fix a `warning` unless you can say why the code is right in that place.

| Severity | Meaning |
| --- | --- |
| `error` | The code gives a wrong result or raises an exception |
| `warning` | The code is slow or easy to get wrong |
| `info` | A shorter or newer form exists |

`xpd check bad.py --fix` writes the safe fixes, which keep what the code does. None of the five
issues has one, so the file stays as it was. `--unsafe-fixes` adds the fixes that may change what
the code does, so read the diff after you use it.

```console
$ xpd check bad.py --fix --unsafe-fixes --format text
bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: fixed 2, 3 issues
```

xpd rewrote lines 6 and 7 and exited with 0, because no error is left.

```diff
 for i, row in df.iterrows():
-    df["a"][i] = row["a"] * 2
-df.dropna(inplace=True)
+    df.loc[i, "a"] = row["a"] * 2
+df = df.dropna()
```

Under pandas copy-on-write, the old line 6 left `df` unchanged, and the new line writes the
doubled values into `df`. `xpd check` exits with 0 when it finds no error, with 1 when it finds
one, and with 2 when it can't run. The
[Get started](https://bjoernaagaard.github.io/xpd/) page explains each part of an issue.

## Animate a concept

Ask an agent with the xpd plugin to animate a concept from the project, or render a
Manim scene directly. The first setup pulls the pinned Cairo renderer into rootless
Podman on x86-64 Linux. Later sessions can use the same renderer and load the guide offline.

```sh
xpd animate setup
xpd animate guide sequence > cache_miss.py
xpd animate render cache_miss.py CacheMiss --profile preview
xpd animate render cache_miss.py CacheMiss --profile final
```

The render result includes a contact sheet, editable source, and saved settings for a revision.
An agent connected over local MCP can use `animate_guide`, `animate_render`,
`animate_artifact`, and `animate_source`. The
[animation guide](https://bjoernaagaard.github.io/xpd/guides/animate/) walks through setup,
CLI and MCP rendering, and how to revise a saved scene.

## Rules

Each library has a pack of rules, and a pack runs only on a file that imports its library. The
Python pack runs on every file. Each rule has a page that says what goes wrong, what to write
instead and whether xpd can fix it, such as
[pandas/chained-assignment](https://bjoernaagaard.github.io/xpd/rules/pandas/chained-assignment/).

| Pack | One of its rules | What the rule reports |
| --- | --- | --- |
| pandas | `chained-assignment` | `df["a"][i] = v` writes to a copy, so the frame keeps its old values |
| NumPy | `append-flattens` | `np.append` without `axis` flattens both arrays |
| SciPy | `removed-keyword` | `tol=` on the sparse iterative solvers was removed and raises `TypeError` |
| scikit-learn | `fit-on-test` | A fit on the test rows scored 0.997 AUC on data with no signal |
| MAPIE | `calibrate-on-training-rows` | `conformalize` on the training rows covered 0.468 of the test rows where 0.9 was asked |
| plotly | `marker-size-diameter` | A marker's `size` is its diameter, so markers of size 10 and 20 had areas of 78.5 and 314.2 square pixels |
| Python | `dict-str-any` | `dict[str, Any]` in a signature hides the keys the function uses |

`xpd rules` lists every rule, and the
[rules overview](https://bjoernaagaard.github.io/xpd/rules/) lists the packs with their counts.

Some `info` rules state a preference of the house style, such as one method chain where the code
builds a frame in separate steps. Set `style` to make xpd report them as warnings, in
`[tool.xpd]` of the project's `pyproject.toml` or at the top of `~/.config/xpd/config.toml`.
[`xpd check`](https://bjoernaagaard.github.io/xpd/commands/check/#house-style) describes the
setting.

```toml
[tool.xpd]
style = "strict"
```

## Commands

`xpd <command> -h` lists the options of a command, and the
[command reference](https://bjoernaagaard.github.io/xpd/commands/) has a page for each one.

```text
xpd
├── lint
│   ├── check [paths]          lint files and folders, and rewrite with --fix
│   ├── rules                  list every rule with its documentation
│   └── hook                   answer an agent's session: lint each edit, and hold a turn that leaves errors or warnings
├── look up a library
│   ├── find "<task>"          find library symbols, rules and worked examples for a task
│   ├── api <name>             read one symbol, method or worked example in full
│   └── index <source>         build a search index from library source
├── read and rewrite code
│   ├── run [paths]            find a code pattern, and preview or apply a rewrite
│   ├── outline [paths]        show imports, definitions and members with line numbers
│   ├── graph <files>          print the dataflow of step code or Hamilton modules
│   └── map <file>             plan a pandas pipeline reimplementation in native Polars
├── run the project's Python
│   ├── profile <data file>    print dtypes, nulls, unique counts and memory per column
│   ├── equiv old.py new.py    run two scripts and compare the `result` each one leaves
│   └── refit <contract>       refit a saved Pipeline on raw training rows and compare
├── animate                    render and revise an editable Manim scene
├── write your own rules
│   ├── new                    create a rule project, a rule, a test case or a utility
│   └── test                   check rule examples and compare snapshots
└── connect a tool
    ├── mcp                    start the MCP server for agents
    ├── lsp                    start the language server for editors
    ├── skill                  print the agent skill for the installed version
    ├── doctor                 suggest Ruff, ty and pytest settings for the project
    └── completions <shell>    print shell completion
```

`profile`, `equiv` and `refit` run the project's Python, so they need pandas in the project's
environment. `index` runs Python with Griffe, and the first `find` downloads a 333 MB search model
once. `check --fix`, `run --update-all`, `new`, and `test -U` write project files when invoked in
those modes, and `hook` records session state in xpd's cache folder.

### Plan a pandas migration

`xpd map features.py --entry add_features --from pandas --to polars` reads the
source and proposes native Polars operations. It shows the stages, observed
columns and behavior that needs more evidence. Use `--format json` for a
structured report. Exit 0 means the plan has no unresolved items, 1 means
the report is partial, and 2 means xpd could not read or analyze the request.

Pass `--contract migration.json` to declare ordered input and output schemas,
positional row identity and the caller's materialization boundary.
Use the same contract with `xpd equiv old.py new.py --contract migration.json`
to compare results from the two scripts. Both scripts must read the same
inputs and assign their tables to `result`. The Polars script returns a
materialized DataFrame in the proof even if the production consumer takes
a LazyFrame. The [equiv reference](https://bjoernaagaard.github.io/xpd/commands/equiv/)
shows the contract fields and exit codes. The [skfolio migration example](examples/skfolio-migration/)
includes two manifests and a replay that verifies native Polars output without pandas
or skfolio installed in the native runtime.

## Use xpd with an agent or an editor

| You want | Do | Guide |
| --- | --- | --- |
| An agent to see the issues of each file it edits, and to keep working while one has an error or a warning | Install the plugin, or add `xpd hook` to the agent's hooks | [Claude Code and other agents](https://bjoernaagaard.github.io/xpd/guides/claude-code/) |
| An agent to call xpd's tools | `claude mcp add xpd -- uvx xpd mcp` | [Connect an agent with MCP](https://bjoernaagaard.github.io/xpd/guides/mcp/) |
| The hook, the MCP server and the skills in one step | Install this repository as a plugin in Claude Code or Codex | [Agent plugin](https://bjoernaagaard.github.io/xpd/guides/claude-code/#agent-plugin) |
| An agent to know when to run each command | `npx skills add bjoernaagaard/xpd` | [Agent skill](https://bjoernaagaard.github.io/xpd/guides/claude-code/#agent-skill) |
| An agent to train a scikit-learn model with checks | `xpd mcp --code-mode` | [Train a model in code mode](https://bjoernaagaard.github.io/xpd/guides/code-mode/) |
| Issues, fixes and rule help in your editor | Start `xpd lsp` from the editor | [Use xpd in an editor](https://bjoernaagaard.github.io/xpd/guides/editors/) |
| A rule for your own project | `xpd new rule` | [Write a rule](https://bjoernaagaard.github.io/xpd/guides/write-a-rule/) |

Install the plugin in Claude Code with the following commands. The guide has the commands for
Codex.

```sh
claude plugin marketplace add bjoernaagaard/xpd
claude plugin install xpd@xpd
```

The plugin runs `xpd hook` at four points of a session.

```text
session starts        xpd prints which rule packs apply, the style setting,
                      and the pipelines and Hamilton nodes of the project
user sends a prompt   xpd prints the playbook that answers the prompt, such as
                      xpd/recipe/plotly-chart for "improve my charts",
                      or the open issues when the prompt asks what to improve
agent edits a file    xpd reports the issues of that file
agent wants to stop   xpd lints every Python file the session changed,
                      and holds the turn while one has an error or a warning
```

With `style = "strict"`, a rule of the house style counts as a warning, so the agent also keeps
working until the code is in the style.

A playbook is one of xpd's own recipes. It holds the house style for a kind of task as a complete
module, with the commands to run in order and the rules that the code satisfies.
`xpd find "<task>"` returns a playbook when the task fits it, and `xpd api <name>` prints it.

| Playbook | What its code is |
| --- | --- |
| `xpd/recipe/pandas-pipeline` | Step functions that each return `frame.assign(...)`, joined with `pipe` |
| `xpd/recipe/feature-module` | One typed function for each feature, named for the column it returns |
| `xpd/recipe/linear-script` | A script that changed a frame line by line, as one chain that `xpd equiv` proves equal |
| `xpd/recipe/plotly-chart` | A line chart that reports no issue of the plotly pack |

## License

You may use xpd's own code under the MIT license or the Apache-2.0 license, and the other parts
that ship with xpd keep the licenses in the table.

| Part | License | Where the text is |
| --- | --- | --- |
| xpd's own code and its playbooks | MIT or Apache-2.0, as you choose | `LICENSE-MIT` and `LICENSE-APACHE` |
| Recipes, which are examples taken from library documentation | The license of the library, which is MIT, BSD-3-Clause, Apache-2.0 or NCSA | `licenses/`, one file per library. `xpd api <recipe>` prints the license name and the source link with the code |
| The Manim Community API index | MIT | [Manim Community](licenses/manim-community-MIT) and [3Blue1Brown](licenses/manim-MIT) |
| `option-ext`, a dependency shipped unmodified | MPL-2.0 | `licenses/option-ext-LICENSE.txt`, with the complete source in `licenses/option-ext-0.2.0-source.tar.gz` and at <https://crates.io/api/v1/crates/option-ext/0.2.0/download> |
| `libcst`, the parser that checks a fixed file | MIT, with parts of CPython under PSF-2.0 | `licenses/libcst-LICENSE` |
| EmbeddingGemma, the model that `find` downloads | The Gemma Terms of Use at <https://ai.google.dev/gemma/terms> | Not shipped with xpd |

The `licenses/` folder ships in the crate, the sdist and the wheel, and the package metadata lists
the combined licenses.
