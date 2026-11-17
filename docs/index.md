# xpd

Run `uvx xpd check .` in your project to find the lines of Python that are slow or give a wrong
result, and add `--fix` to rewrite each line whose fix keeps what the code does. xpd checks code
that uses pandas, NumPy, SciPy, scikit-learn, scikit-lego, MAPIE, Category Encoders,
linearmodels, arch, pymoo, plotly or Apache Hamilton.

```console
$ uvx xpd check samples/bad.py
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
samples/bad.py:7:1: info [pandas/no-inplace] inplace=True gives no speedup and blocks chaining. Reassign instead.
samples/bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
samples/bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: 5 issues, 2 more with --fix --unsafe-fixes
```

## Install

`uv tool install xpd` installs the `xpd` command, and `uvx xpd check .` runs it once without an
install.

```sh
uv tool install xpd
```

## Read the output

Fix every `error`, and fix a `warning` unless you can say why the code is right in that place.

```text
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy ...
│              │ │  │      │                          └── what goes wrong, and what to write instead
│              │ │  │      └── the pack and the rule, which `xpd rules` documents
│              │ │  └── error, warning or info
│              │ └── column
│              └── line
└── file
```

| Severity | Meaning |
| --- | --- |
| `error` | The code gives a wrong result or raises. |
| `warning` | The code is slow or easy to get wrong. |
| `info` | A shorter or newer form exists. |

## Let xpd fix the lines

`xpd check . --fix` writes each fix that keeps what the code does. Add `--unsafe-fixes` to also
write each fix that may change what the code does, and read the diff afterwards. On
`samples/bad.py` the two flags rewrite two of the five lines and report the other three again.

```diff
 for i, row in df.iterrows():
-    df["a"][i] = row["a"] * 2
-df.dropna(inplace=True)
+    df.loc[i, "a"] = row["a"] * 2
+df = df.dropna()
```

```console
$ xpd check samples/bad.py --fix --unsafe-fixes
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
samples/bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: fixed 2, 3 issues
```

## How xpd runs

xpd reads the files and starts no Python, so a coding agent can run it after every edit.

```text
xpd check .
  for each .py file, in parallel
    parse the file once with ast-grep
    read which libraries the file imports
    run the pack of each imported library, and the Python pack
  print one line per issue, and a summary
```

## Where to go next

Pick the page for what you want to do next.

| You want to | Page |
| --- | --- |
| see every command, its output and its exit codes | [Commands](commands.md) |
| lint each edit that Claude Code makes, or load xpd as an agent plugin | [Claude Code](claude-code.md) |
| give an agent typed tools, resources and prompts | [MCP server](mcp.md) |
| take a data file to a saved scikit-learn model with checks that xpd runs | [Code mode](code-mode.md) |
| see issues, fixes and rule documentation in an editor | [Language server](lsp.md) |
| read every rule with code before and after, or write your own | [Rules](rules.md) |
