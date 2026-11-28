Remove a library from `[tool.xpd.libraries]` in `pyproject.toml` and delete its managed `.xpd/indexes/<package>.json` file unless `--keep-index` is given.

## Example

```toml title="pyproject.toml"
[project]
name = "demo"
version = "0.1.0"

[tool.xpd.libraries]
mypkg = ".xpd/indexes/mypkg.json"
```

```console
$ xpd library remove mypkg --keep-index --format json | jq -c '{package}'
{"package":"mypkg"}
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The library was removed from `pyproject.toml` |
| 2 | No `pyproject.toml` or matching `[tool.xpd.libraries]` entry was found |
