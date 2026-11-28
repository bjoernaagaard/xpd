Index a third-party or local Python package from source without importing it, cache its `EmbeddingGemma` vectors locally, and register it under `[tool.xpd.libraries]` in `pyproject.toml`.

## Examples

```sh
xpd library add pydantic
xpd library add skpower --source src --scaffold-rules
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The index was written, cached, and recorded in `pyproject.toml` |
| 2 | The package source could not be located or indexed |
