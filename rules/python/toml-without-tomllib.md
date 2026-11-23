The standard library reads TOML since Python 3.11. Use `tomllib` in place of `tomli` or `toml.load`.

```diff lang="python"
-import toml
+import tomllib
 
 
 def settings(path: Path) -> dict[str, object]:
-    return toml.load(path)
+    return tomllib.loads(path.read_text())
```

## What the rule reports

The rule reports an import of `tomli`, and `toml.load` and `toml.loads`. It leaves alone
`toml.dump` and `toml.dumps`, because `tomllib` cannot write, and an import of `tomli` inside
`try` or `if`, which is how code that still runs on Python 3.10 falls back.

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites `from tomli import ...` and `import tomli as ...` to `tomllib`. Bare `import tomli` and `toml.load`/`toml.loads` calls remain manual because `tomllib.load` requires a binary file.

[`tomllib`](https://docs.python.org/3/library/tomllib.html) has been in the standard library
since Python 3.11, and every Python that still gets fixes has it. `tomli` is the package that
`tomllib` was copied from, with the same `load` and `loads`, so only the import changes.
`toml.load` takes a path or a file opened as text, and `tomllib.load` takes a file opened with
`"rb"`, so pass the text of the file to `tomllib.loads` or open the file in binary mode.
