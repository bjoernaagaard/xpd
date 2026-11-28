Remove a library from `[tool.xpd.libraries]` in `pyproject.toml` and delete its managed `.xpd/indexes/<package>.json` file unless `--keep-index` is given.

## Examples

```sh
xpd library remove pydantic
xpd library remove skpower --keep-index
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The library was removed from `pyproject.toml` |
| 2 | No `pyproject.toml` or matching `[tool.xpd.libraries]` entry was found |
