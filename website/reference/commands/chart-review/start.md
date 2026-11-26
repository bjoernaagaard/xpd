Start a recorded chart and table review run. xpd records the commit that is checked out, the paths, each `--check` command, and an optional `--adapter` (a Python file defining `figures()` or a shell command printing normalized figure JSON). Give a check for each command the project checks itself with. A check that fails at the start gates nothing. The tracked files must match the commit, and a run that is not finished is kept unless you pass `--restart`.

The [`xpd chart-review`](../../chart-review/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd chart-review start --help
...
```
