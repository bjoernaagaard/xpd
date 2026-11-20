Use `check` after the agent writes or changes Python, as a person runs
[`xpd check`](../../../commands/check/). It lints files and folders and returns every issue with its rule,
place, severity and fix. It changes no file, so a client can run it without asking.

## Example

The call below lints one file for one rule, and counts the unsafe fix in `fixable`.

```json
{"paths":["samples/bad.py"],"unsafe_fixes":true,"options":{"rules":["pandas/chained-assignment"]}}
```

```json
{"fixed":0,"fixable":1,"issues":[{"package":"pandas","rule":"chained-assignment","file":"samples/bad.py","line":6,"column":5,"severity":"error","message":"Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, \"a\"] instead.","fix":"unsafe"}]}
```

## Errors

A path outside the project root fails before xpd runs.

```text
../outside.py is outside the project root /tmp/project. Pass a path inside it.
```

A failure of the binary, such as a file it cannot read, comes back as a tool error with the
command and its message. Issues are a result, not an error.
