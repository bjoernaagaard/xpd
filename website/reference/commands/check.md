Use `xpd check` after you write or change Python. It lints the files and folders you name, or the
current folder when you name none, and prints one line for each issue. In a folder, xpd skips the
files that `.gitignore` lists. The [rules](../../rules/) pages explain each issue.

## Examples

```console
$ xpd check samples/bad.py --unsafe-fixes --format text
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] [*] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
samples/bad.py:7:1: info [pandas/no-inplace] [*] inplace=True gives no speedup and blocks chaining. Reassign instead.
samples/bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
samples/bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: 5 issues, 2 fixable with --fix --unsafe-fixes
```

Each line gives the place, the severity, the rule and the message. `[*]` marks an issue that
`--fix` fixes with the flags you passed. The summary line goes to stderr.

Which rules run on a file depends on what the file imports and on what the project's Python has
installed.

| Rules | Run on |
| --- | --- |
| The rules of a library, such as `pandas/row-loop` | Files that import the library |
| The Python rules | Every file |
| A rule for a newer library version than the project's Python has | No file, because xpd skips the rule |
| A package check, such as `pandas/missing-pyarrow` | The project, once per run, when the project's Python has the library and lacks the package |

## Fixes

`--fix` writes the safe fixes and then lints what is left, and `--unsafe-fixes` adds the unsafe
fixes. A fix is unsafe when it may change what the code does, for example when another name still
points at the old object.

| Flags | Effect |
| --- | --- |
| none | Lint. `[*]` marks an issue with a safe fix |
| `--fix` | Write the safe fixes, then lint what is left |
| `--unsafe-fixes` | Lint. `[*]` also marks an issue with an unsafe fix |
| `--fix --unsafe-fixes` | Write every fix, then lint what is left |
| `--fix --review` | Show each safe fix and ask before writing it |
| `--fix --review --unsafe-fixes` | Show each safe and unsafe fix and ask before writing it |

## JSON output

`--format json` prints `fixed`, `fixable` and `issues`. `fixed` counts the fixes that `--fix`
wrote, and `fixable` counts the issues that `--fix` would fix with the same `--unsafe-fixes`
setting. In each issue, `fix` is `safe`, `unsafe`, or `null` when the rule has no fix. A package
check is about the project and not about a place in a file, so its `line` and `column` are
`null`.

```console
$ xpd check samples/bad.py --format json | jq -c '{fixed, fixable, issue: .issues[1]}'
{"fixed":0,"fixable":0,"issue":{"column":5,"file":"samples/bad.py","fix":"unsafe","line":6,"message":"Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, \"a\"] instead.","package":"pandas","rule":"chained-assignment","severity":"error"}}
```

`--format sarif` prints SARIF 2.1.0, the format that code scanning tools read. `--format github`
prints one GitHub Actions workflow command for each issue, so the issues show on the pull request.

## Project rules

xpd adds your own rules from the nearest `sgconfig.yml` or `sgconfig.yaml` at or above the
current folder, or from the file that `--config` names. [`xpd new project`](../new/project/)
writes one.

| File or key | What it holds |
| --- | --- |
| `ruleDirs` in `sgconfig.yml` | Plain ast-grep rules, for any language |
| `utilDirs` in `sgconfig.yml` | Utilities that rules use with `matches: <name>` |
| `xpd.yml` beside `sgconfig.yml` | Library packs, each with a Python module and its own `ruleDirs` |

A rule in a library pack runs only on files that import the pack's module, and only for the
library versions that its `since` and `until` name. A project rule gets an unsafe fix unless its
`metadata.fix` says `safe`. xpd warns when `sgconfig.yml` sets `languageGlobs` or
`languageInjections`, and it skips the rules of `customLanguages`.

## House style

Some rules state a preference about code that works, such as one method chain where the code
builds a frame in separate steps. They are `info`, and their pages call them rules of the house
style. Set `style` to `"strict"` to make xpd report them as warnings, so that
[`xpd hook`](../hook/) holds a turn that leaves one.

```toml
# pyproject.toml
[tool.xpd]
style = "strict"
```

xpd reads the setting from two files, and the first one that sets it wins.

| File | Where the key goes |
| --- | --- |
| The closest `pyproject.toml` at or above the current folder that has a `[tool.xpd]` table | `style` in `[tool.xpd]` |
| `xpd/config.toml` in `$XDG_CONFIG_HOME`, which is `~/.config` when the variable is not set, and `%APPDATA%` on Windows | `style` at the top of the file |

The values are `"strict"` and `"default"`. A project sets `"default"` to turn off a `"strict"`
that the user's file sets. xpd exits with 2 and names the file when the value is another one, or
when the file sets a key that xpd does not read.

The example runs xpd on a `script.py` that filters one frame in three statements, first without
the setting and then with it.

```python
import pandas as pd

df = pd.read_csv("orders.csv")
df = df[df["price"] > 0]
df = df[df["qty"] >= 2]
df = df[df["city"] != "Oslo"]
```

```console
$ xpd check script.py --format text
script.py:4:1: info [pandas/filter-ladder] df is filtered three or more times in a row, each time into itself, so every line leaves a half-filtered frame under the final name. Write one chain, as in df = df.loc[lambda d: ...].loc[lambda d: ...].loc[lambda d: ...].
xpd: 1 issue
$ printf '[tool.xpd]\nstyle = "strict"\n' > pyproject.toml
$ xpd check script.py --format text
script.py:4:1: warning [pandas/filter-ladder] df is filtered three or more times in a row, each time into itself, so every line leaves a half-filtered frame under the final name. Write one chain, as in df = df.loc[lambda d: ...].loc[lambda d: ...].loc[lambda d: ...].
xpd: 1 issue
```

The setting changes the severity in each format of `xpd check`, in the hook, in the language
server and in the MCP server. `xpd rules --format json` lists the rules of the house
style with `"style": "house"`.

## Suppress an issue

A comment `# ast-grep-ignore: pandas/row-loop` turns the named rule off for the next line, or for
its own line when it follows code. A comment without a rule name turns off every rule.

```python
# ast-grep-ignore: pandas/row-loop
for i, row in df.iterrows():
    pass
```

xpd reports a comment that suppresses nothing as
[`xpd/unused-suppression`](../../rules/xpd/unused-suppression/). A run with `--rule` or
`--severity` skips that check, because a rule it left out could use the comment.

## Exit status

The format never changes the exit code or the rules that run.

| Exit code | Meaning |
| --- | --- |
| 0 | No issue of the `--fail-on` severity or above is left, after fixes |
| 1 | At least one issue of the `--fail-on` severity or above is left. The default is `error` |
| 2 | xpd could not run, or it could not read a file. It names that file on stderr and still reports the others |
