Index a third-party or local Python package from source without importing it, cache its `EmbeddingGemma` vectors locally, and register it under `[tool.xpd.libraries]` in `pyproject.toml`.

## Examples

```sh
xpd library add pydantic
xpd library add skpower --source src --scaffold-rules
```

Without Griffe or `uv` on `PATH`, `xpd library add` prints the command needed to install Griffe and exits with `2`.

```toml title="pyproject.toml"
[project]
name = "demo"
version = "0.1.0"
```

```python title="mypkg/__init__.py"
def greet(name: str = "world") -> str:
    """Return a greeting."""
    return f"hello {name}"
```

```console
$ xpd library add mypkg --source . --version 0.1.0 --no-embed
xpd: Index building needs griffe==2.4.1. Run uv run --with griffe==2.4.1 xpd index /tmp/project --package mypkg --version 0.1.0.
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The index was written, cached, and recorded in `pyproject.toml` |
| 2 | Griffe is missing, or the package source could not be located or indexed |
