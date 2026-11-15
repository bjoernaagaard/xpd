# xpd

xpd checks Python code for slow operations and wrong results. It checks code that uses pandas,
NumPy, SciPy, scikit-learn or Apache Hamilton. xpd can rewrite many of the lines it reports, and
a coding agent can run it after every edit.

## What xpd reports

`samples/bad.py` holds slow loops and pandas assignments that write to a copy. The diff below
shows what `xpd check samples/bad.py --fix --unsafe-fixes` changes. Each comment names the rule
that reports the line.

```diff
 import pandas as pd

 df = pd.DataFrame({"a": [1, 2], "k": [1, 2]})
 other = df.copy()
 for i, row in df.iterrows():           # warning  pandas/row-loop
-    df["a"][i] = row["a"] * 2          # error    pandas/chained-assignment
+    df.loc[i, "a"] = row["a"] * 2
-df.dropna(inplace=True)                # info     pandas/no-inplace
+df = df.dropna()
 m = pd.merge(df, other, on="k")        # warning  pandas/merge-without-validate
 out = pd.DataFrame()
 for f in ["x.csv"]:
     out = pd.concat([out, pd.read_csv(f)])  # warning  pandas/concat-in-loop
```

Each message says what goes wrong and what to write instead.

```text
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
```

An issue has one of three severities, and a fix is either safe or unsafe.

| Word | Meaning |
| --- | --- |
| `error` | The code gives a wrong result or raises. Fix it. |
| `warning` | The code is slow or easy to get wrong. Fix it unless you can say why it is right here. |
| `info` | A shorter or newer form exists. |
| safe fix | Keeps what the code does. `--fix` writes it. |
| unsafe fix | May change what the code does. `--fix --unsafe-fixes` writes it, so read the diff afterwards. |

## Rules

Each library has one pack of rules, written for the library's current version. A library's pack
runs only on files that import the library. The Python pack runs on every file.

| Pack | One of its rules | What the rule reports |
| --- | --- | --- |
| pandas | `chained-assignment` | `df["a"][i] = v` writes to a copy, so the frame keeps its old values |
| NumPy | `append-flattens` | `np.append` without `axis` flattens both arrays |
| SciPy | `removed-keyword` | `tol=` on the sparse iterative solvers was removed and raises `TypeError` |
| scikit-learn | `fit-on-test` | A fit on the test rows makes the score on those rows useless |
| Hamilton | `node-mutates-input` | A node writes to one of its parameters, and Hamilton hands the same object to other nodes |
| Python | `dict-str-any` | `dict[str, Any]` in a signature hides the keys the function uses |

`xpd rules` lists every rule, and the [rules page](https://bjoernaagaard.github.io/xpd/rules/)
shows each one with code before and after.

## Install

```sh
uv tool install xpd
```

`uvx xpd check .` runs it once without an install.

## Commands

```text
xpd
├── check [paths]          lint files and folders, and rewrite with --fix
├── rules                  list every rule with its documentation
├── graph <files>          print the dataflow of step code or Hamilton modules
├── profile <data file>    print dtypes, nulls, unique counts and memory per column
├── equiv old.py new.py    run two scripts and compare the `result` each one leaves
├── hook                   lint the file that a Claude Code edit touched
├── mcp                    start the MCP server
└── skill                  print the agent skill for this version
```

`check`, `rules` and `graph` read the files and start no Python. `profile` and `equiv` run the
project's Python and need pandas there. `xpd <command> -h` lists the flags of a command. The
[commands page](https://bjoernaagaard.github.io/xpd/commands/) explains the output and the exit
codes.

## Use with a coding agent

An agent can use xpd as a plugin, or through a hook, an MCP server and skills that you set up
one at a time.

| Way | What the agent gets | Setup |
| --- | --- | --- |
| Plugin | The hook, the MCP server and the skills together | Load this repository in a client that supports Agent Plugins |
| Hook | The issues of each file, right after the edit | `xpd hook` in the Claude Code settings |
| MCP server | Typed tools, the rules, the skills and prompts | `xpd mcp` |
| Skills | When to run each command, and Python style that a linter cannot check | `npx skills add bjoernaagaard/xpd` |

### Plugin

This repository is a plugin in the [Agent Plugins](https://github.com/agentplugins/agent-plugins-spec)
format, version 1.1.0. A client that supports the format loads the folder and gets everything
below. The plugin starts xpd with `uvx`, so the machine needs [uv](https://docs.astral.sh/uv/).

```text
xpd, loaded as a plugin
├── plugin.json    the name and version of the plugin
├── skills/        xpd, python-best-practices and python-review
├── mcp.json       starts the MCP server of this version with uvx
└── hooks.json     runs the hook of this version with uvx after each tool call
```

### Hook

After each edit to a `.py` file, Claude Code sends the edit to `xpd hook`. When xpd finds
issues, Claude Code shows them to the agent in the same turn.

```text
Claude Code edits features.py
└── the PostToolUse hook runs xpd hook
    ├── no issues: exit code 0
    └── issues: exit code 2
        └── Claude Code shows the issues to the agent
```

Add the hook to `.claude/settings.json` in your project.

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

### MCP server

`xpd mcp` starts an [MCP](https://modelcontextprotocol.io) server, so an agent calls xpd through
typed tools and needs no shell. The server comes with xpd, and the command below adds it to
Claude Code.

```sh
claude mcp add xpd -- uvx xpd mcp
```

```text
xpd mcp
├── tools
│   ├── check, fix                 lint, and write fixes after the user confirms unsafe ones
│   ├── graph, profile, equiv      the commands of the same names
│   ├── rules, rule                list the rules, and read one with its examples
│   └── show_issues, show_dataflow, show_columns    tables and a chart, in hosts that draw them
├── resources
│   ├── xpd://rules/{package}/{rule}    one rule's documentation
│   └── skill://<name>/SKILL.md         each skill that ships with xpd
└── prompts
    └── lint_and_fix, review, refactor_data_logic, add_to_dataflow
```

The server takes every rule and every result from the xpd binary, so it matches the installed
version. The [MCP page](https://bjoernaagaard.github.io/xpd/mcp/) covers other clients, the
settings and HTTP.

### Skills

The repository includes [Agent Skills](https://agentskills.io) in `skills/`. The Python skills
work without xpd. They also run xpd when it is installed.

| Skill | What it does |
| --- | --- |
| `xpd` | Says when to run each xpd command. `xpd skill` prints it for the installed version. |
| `python-best-practices` | Holds the rules an agent follows when it writes or changes Python. |
| `python-review` | Reviews a file, package or diff, and cites the rule for every finding. |

```sh
npx skills add bjoernaagaard/xpd
```

## Documentation

The full guide is at <https://bjoernaagaard.github.io/xpd/>. It also explains how to write a
rule.

## License

You may use xpd under the MIT license or the Apache-2.0 license.
