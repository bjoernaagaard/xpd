# xpd

Run `uvx xpd check .` to find the lines of your Python code that are slow or give a wrong
result, and add `--fix` to let xpd rewrite the ones it can. xpd checks code that uses pandas,
NumPy, SciPy, scikit-learn, scikit-lego, MAPIE, Category Encoders, linearmodels, arch, pymoo,
plotly or Apache Hamilton. It starts no Python, so a coding agent can run it after every edit.

```console
$ uvx xpd check samples/bad.py
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
samples/bad.py:7:1: info [pandas/no-inplace] inplace=True gives no speedup and blocks chaining. Reassign instead.
samples/bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
samples/bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: 5 issues, 2 more with --fix --unsafe-fixes
```

## What a fix changes

`xpd check samples/bad.py --fix --unsafe-fixes` rewrites two of the five lines and reports the
other three again.

```diff
 import pandas as pd

 df = pd.DataFrame({"a": [1, 2], "k": [1, 2]})
 other = df.copy()
 for i, row in df.iterrows():
-    df["a"][i] = row["a"] * 2
-df.dropna(inplace=True)
+    df.loc[i, "a"] = row["a"] * 2
+df = df.dropna()
 m = pd.merge(df, other, on="k")
 out = pd.DataFrame()
 for f in ["x.csv"]:
     out = pd.concat([out, pd.read_csv(f)])
```

Fix every `error`, and fix a `warning` unless you can say why the code is right in that place.

| Word | Meaning |
| --- | --- |
| `error` | The code gives a wrong result or raises. |
| `warning` | The code is slow or easy to get wrong. |
| `info` | A shorter or newer form exists. |
| safe fix | The rewrite keeps what the code does, and `--fix` writes it. |
| unsafe fix | The rewrite may change what the code does. `--fix --unsafe-fixes` writes it, so read the diff afterwards. |

`xpd check` exits with 0 when it finds no `error`, with 1 when it finds one, and with 2 when it
can't run.

## Install

`uv tool install xpd` installs the `xpd` command, and `uvx xpd check .` runs it once without an
install.

```sh
uv tool install xpd
```

## Rules

Each library has one pack of rules, and the pack runs only on a file that imports the library.
The Python pack runs on every file. The plotly pack also runs on a file that makes plotly the
plotting backend of pandas.

| Pack | One of its rules | What the rule reports |
| --- | --- | --- |
| pandas | `chained-assignment` | `df["a"][i] = v` writes to a copy, so the frame keeps its old values |
| NumPy | `append-flattens` | `np.append` without `axis` flattens both arrays |
| SciPy | `removed-keyword` | `tol=` on the sparse iterative solvers was removed and raises `TypeError` |
| scikit-learn | `fit-on-test` | A fit on the test rows scored 0.997 AUC on data with no signal |
| scikit-lego | `thresholder-score` | `Thresholder.score` ignores `threshold`, so a grid search over 0.1, 0.5 and 0.9 gave 0.875 three times |
| MAPIE | `calibrate-on-training-rows` | `conformalize` on the training rows covered 0.483 of the test rows where 0.9 was asked |
| Category Encoders | `ordinal-mapping-pairs` | An `OrdinalEncoder` mapping given as a list of pairs codes every row as -1.0 |
| linearmodels | `clustered-without-clusters` | `cov_type="clustered"` without clusters gives the robust standard errors |
| arch | `unscaled-returns` | Returns as decimals stopped the optimizer with code 4 and gave a mean of 4.02 where 0.056 was right |
| pymoo | `objectives-stacked-by-row` | `out["F"] = np.array([f1, f2])` in a `Problem` gives each solution the wrong values |
| plotly | `marker-size-diameter` | A marker's `size` is its diameter, so markers of size 10 and 20 had areas of 78.5 and 314.2 square pixels |
| Hamilton | `node-mutates-input` | A node writes to its input, and Hamilton hands the same object to other nodes |
| Python | `dict-str-any` | `dict[str, Any]` in a signature hides the keys the function uses |

`xpd rules` lists every rule, and the [rules page](https://bjoernaagaard.github.io/xpd/rules.html)
shows each one with code before and after.

## Commands

`xpd <command> -h` lists the flags of a command, and the
[commands page](https://bjoernaagaard.github.io/xpd/commands.html) shows the output and the exit
codes of each one.

```text
xpd
├── lint
│   ├── check [paths]          lint files and folders, and rewrite with --fix (alias: scan)
│   ├── rules                  list every rule with its documentation
│   └── hook                   lint the files that an agent's edit changed
├── look up a library
│   ├── find "<task>"          find library symbols, rules and worked recipes for a task
│   ├── api <name>             read one symbol, method or recipe in full
│   └── index <source>         build a search index from library source
├── read and rewrite code
│   ├── run [paths]            find a code pattern, and preview or apply a rewrite (alias: query)
│   ├── outline [paths]        show imports, definitions and members with line numbers
│   └── graph <files>          print the dataflow of step code or Hamilton modules
├── run the project's Python
│   ├── profile <data file>    print dtypes, nulls, unique counts and memory per column
│   ├── equiv old.py new.py    run two scripts and compare the `result` each one leaves
│   └── refit <contract>       refit a saved Pipeline on raw training rows and compare
├── write your own rules
│   ├── new                    create a rule project, a rule, a test case or a utility
│   └── test                   check rule examples and compare snapshots
└── connect a tool
    ├── mcp                    start the MCP server for agents
    ├── lsp                    start the language server for editors
    ├── skill                  print the agent skill for the installed version
    └── completions <shell>    print shell completion for bash, elvish, fish, powershell or zsh
```

Only `profile`, `equiv` and `refit` start Python, and they need pandas in the project's
environment. Every other command reads files.

## Look up a library you don't know

`xpd find "<task>"` names the classes, the rules and the worked examples for a task in your own
words, also for a library you haven't installed. `xpd api <name>` then prints one of them in
full, with each parameter as the source writes it, so you see the type annotation and the
default. A stub file writes `...` for a default. The name can also be a method of a class, such
as `Class.method`.

```console
$ xpd find "time series bootstrap for cross validation" --limit 1
Symbols
sklearn.model_selection.TimeSeriesSplit(n_splits, max_train_size, test_size, gap) [class]
Time Series cross-validator.
Rules: sklearn/groups-ignored, sklearn/removed-in-sklearn
Rules
sklego/group-time-series-split-folds [rule]
The rule reports `GroupTimeSeriesSplit` with 6 or more splits. The class tries every combination of
split points over the groups to find the most even folds, so each extra fold multiplies the time.
Recipes
mapie/recipe/generate-a-sampling-method-that-block-bootstraps-the [recipe]
Generate a sampling method, that block bootstraps the training set.

$ xpd api mapie.subsample.BlockBootstrap
mapie.subsample.BlockBootstrap(n_resamplings: int = 30, length: Optional[int] = None, n_blocks: Optional[int] = None, overlapping: bool = False, random_state: Optional[Union[int, RandomState]] = None) [class]
Generate a sampling method, that block bootstraps the training set. It can replace KFold, LeaveOneOut or SubSample as cv argument in the TimeSeriesRegressor class.
  get_n_splits(args, kargs) Returns the number of splitting iterations in the cross-validator.
  split(X, args, kargs) Generate indices to split data into training and test sets.
Rules: mapie/unseeded-resampling

$ xpd api mapie.subsample.BlockBootstrap.split
mapie.subsample.BlockBootstrap.split(X: NDArray, *args: Any, **kargs: Any) [method]
Generate indices to split data into training and test sets.
```

| Fact about `find` | Value |
| --- | --- |
| Results without `--limit` | Up to 5 symbols, 3 rules and 3 recipes |
| Flags | `--kind symbol`, `--kind rule` or `--kind recipe` selects one kind, and `--package` selects one library |
| Ranking | By keywords and by meaning, so a task finds an entry whose documentation uses other words |
| Model for the meaning | EmbeddingGemma, which the first `find` downloads once, 333 MB, to `~/.cache/xpd/models` |
| Without the model | `find` says so on stderr and ranks by keywords only |
| Where recipes come from | Library documentation and galleries. xpd keeps an example only when its own rules report nothing in it, and it never runs the example |

On macOS the model folder is `~/Library/Caches/xpd/models`, and on Windows it is
`%LOCALAPPDATA%\xpd\models`. With `XDG_CACHE_HOME` set it is `$XDG_CACHE_HOME/xpd/models`.

## Use in an editor

`xpd lsp` starts a language server, so your editor shows each issue while you type, with a quick
fix and the rule's documentation on hover. The
[language server page](https://bjoernaagaard.github.io/xpd/lsp.html) has the setup for Neovim, Helix,
Zed and VS Code.

## Use with a coding agent

Pick the plugin when your client supports it, because it sets up the MCP server and the skills
together. Otherwise set up the parts you want one at a time.

| Way | What the agent gets | Setup |
| --- | --- | --- |
| Plugin | The MCP server and the skills together | Load this repository in a client that supports Agent Plugins, such as Codex |
| Hook | The issues of each file, right after the edit | `xpd hook` in the Claude Code settings |
| MCP server | Typed tools, the rules, the skills and prompts | `xpd mcp` |
| Skills | When to run each command, and Python style that a linter cannot check | `npx skills add bjoernaagaard/xpd` |

### Plugin

Codex installs the plugin with two commands. The plugin starts xpd with `uvx`, so the machine
needs [uv](https://docs.astral.sh/uv/).

```sh
codex plugin marketplace add bjoernaagaard/xpd
codex plugin add xpd@xpd
```

The repository is a plugin in the
[Agent Plugins](https://github.com/agentplugins/agent-plugins-spec) format, version 1.0.0. Version
1.0.0 has no hooks, so you add the hook below yourself.

```text
xpd, loaded as a plugin
├── plugin.json                         the name and version of the plugin
├── skills/                             xpd, python-best-practices and python-review
├── mcp.json                            starts the MCP server of this version with uvx
└── .agents/plugins/marketplace.json    lists the plugin for Codex
```

### Hook

Add the hook below to `.claude/settings.json` in your project, and Claude Code shows the agent
the issues of each `.py` file it edits, in the same turn.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [{ "type": "command", "command": "xpd hook" }]
      }
    ]
  }
}
```

```text
Claude Code edits features.py
└── the PostToolUse hook runs xpd hook
    ├── no issues: exit code 0
    ├── issues: exit code 2, and Claude Code shows them to the agent
    └── xpd can't run: exit code 1, and Claude Code shows the message to you
```

### MCP server

The command below adds the [MCP](https://modelcontextprotocol.io) server to Claude Code, so the
agent calls xpd through typed tools and needs no shell.

```sh
claude mcp add xpd -- uvx xpd mcp
```

```text
xpd mcp
├── tools
│   ├── check, fix                         lint, and write fixes after the user confirms unsafe ones
│   ├── graph, profile, equiv, refit       the commands of the same names
│   ├── find, api                          look up a library for a task and read its API
│   ├── query, rewrite                     search code, preview replacements and confirm edits
│   ├── outline                            read the project's symbols and imports
│   ├── new, test                          create rules, and check their examples and snapshots
│   ├── rules, rule                        list the rules, and read one with its examples
│   ├── discovery_gate, contract, gates    judge a run from a data file to a saved model
│   └── show_issues, show_dataflow, show_columns    tables and a chart, in hosts that draw them
├── resources
│   ├── xpd://rules/{package}/{rule}       one rule's documentation
│   └── skill://<name>/SKILL.md            each skill that ships with xpd
└── prompts
    └── lint_and_fix, review, refactor_data_logic, add_to_dataflow, build_pipeline
```

The server takes every rule and every result from the xpd binary, so it matches the installed
version. The [MCP page](https://bjoernaagaard.github.io/xpd/mcp.html) covers other clients, the
settings and HTTP. The [code mode page](https://bjoernaagaard.github.io/xpd/code-mode.html) shows a
run from a data file to a saved scikit-learn model, with checks that xpd runs itself.

### Skills

The command below installs the three [Agent Skills](https://agentskills.io) in `skills/`. The
two Python skills work without xpd, and they run xpd when it is installed.

```sh
npx skills add bjoernaagaard/xpd
```

| Skill | What it does |
| --- | --- |
| `xpd` | Says when to run each xpd command. `xpd skill` prints it for the installed version. |
| `python-best-practices` | Holds the rules an agent follows when it writes or changes Python. |
| `python-review` | Reviews a file, package or diff, and cites the rule for every finding. |

## Documentation

The full guide is at <https://bjoernaagaard.github.io/xpd/>, and its rules page also explains
how to write a rule.

## License

You may use xpd's own code under the MIT license or the Apache-2.0 license. The table lists
what else ships with xpd.

| Part | License | Where the text is |
| --- | --- | --- |
| xpd's own code | MIT or Apache-2.0, as you choose | `LICENSE-MIT` and `LICENSE-APACHE` |
| Recipes, which are examples taken from library documentation | The license of the library, which is MIT, BSD-3-Clause, Apache-2.0 or NCSA | `licenses/`, one file per library. `xpd api <recipe>` prints the license name and the source link with the code |
| `option-ext`, a dependency shipped unmodified | MPL-2.0 | `licenses/option-ext-LICENSE.txt`, with the complete source in `licenses/option-ext-0.2.0-source.tar.gz` and at <https://crates.io/api/v1/crates/option-ext/0.2.0/download> |
| EmbeddingGemma, the model that `find` downloads | The Gemma Terms of Use at <https://ai.google.dev/gemma/terms> | Not shipped with xpd |

The `licenses/` folder ships in the crate, the sdist and the wheel, and the package metadata
lists the combined licenses.
