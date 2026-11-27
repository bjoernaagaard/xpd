Use `xpd model-review finish` in a [recorded model review](../../model-review/#recorded-stages).

Finish exits 1 while a fix is open, a current question remains unanswered or a new finding lacks a reason. Resolve outstanding items through `xpd model-review plan` and the gate before closing the run.

## Example

```console
$ xpd model-review finish --help
End the run: compare findings, line count and benchmark timing with the start commit. Exit 1 while a fix is open or a new finding has no reason
...
```
