Use `xpd model-review start` in a [recorded model review](../../model-review/#recorded-stages).

Commit the selected files and any Python adapter. Pass the project checks with repeated `--check` options. The run stores its starting commit and adapter.

## Example

```console
$ xpd model-review start --help
Start a run: record the commit it starts from, the paths, line count, checks and optional benchmark of the project
...
```
