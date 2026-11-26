Start a refactor run. xpd records the commit that is checked out, the paths, the code line count, each `--check` command, and an optional `--bench` workload command (timed with 1 warmup and `--bench-runs` measured runs). Give a check for each command the project checks itself with. A check that fails at the start gates nothing. The tracked files must match the commit, and a run that is not finished is kept unless you pass `--restart`.

The [`xpd optimize`](../../optimize/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd optimize start --help
...
```
