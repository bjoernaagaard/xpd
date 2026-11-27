Use `xpd ts-review gate` in the [recorded time series review](../../ts-review/#a-recorded-run).

Gate one committed fix on project checks and the recorded findings. A design fix must edit a file that owns a named checklist item.

## Example

```console
$ xpd ts-review gate --help
Gate a fix that is committed: run the checks, time `--bench` when set, count lines and compare the findings with the commit before it. Exit 1 when the fix is not kept
...
```
