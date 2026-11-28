Manage local and third-party Python library indexes configured under `[tool.xpd.libraries]` in `pyproject.toml`.

Indexes added with `xpd library add` layer cleanly alongside xpd's bundled libraries in `xpd find`, `xpd api`, `xpd hook`, `xpd mcp`, `xpd gates`, and `xpd untangle`.

## Examples

```console
$ xpd library --help
...
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The subcommand completed |
| 2 | Configuration or indexing failed |
