Start a refactor run. xpd records the commit that is checked out, the paths and each `--check` command, runs each check once, and counts the findings. Give a check for each command the project checks itself with. A check that fails at the start gates nothing. The tracked files must match the commit, and a run that is not finished is kept unless you pass `--restart`.

The [`xpd untangle`](../../untangle/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd untangle start --help
...
```
