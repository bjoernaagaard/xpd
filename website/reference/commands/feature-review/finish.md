Use `xpd feature-review finish` in the [recorded feature review](../../feature-review/#recorded-stages).

Finish exits 1 for an open fix, unexplained new findings or unanswered current questions. Exit 0 records a finished run. Adapter and protocol errors stop the stage.

## Example

```console
$ xpd feature-review finish --help
End the run: compare findings, line count and benchmark timing with the start commit. Exit 1 while a fix is open or a new finding has no reason
...
```
