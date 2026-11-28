List the project libraries configured under `[tool.xpd.libraries]` in `pyproject.toml`, with symbol and cached vector counts. Pass `--all` to include xpd's bundled libraries.

## Examples

```console
$ xpd library list --all --format json | jq -c 'length > 0'
true
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | Libraries were listed |
| 2 | A configured index could not be read |
