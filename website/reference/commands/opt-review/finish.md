Use `xpd opt-review finish` during a [recorded optimization review](../../opt-review/#recorded-stages).

Finish after all fixes are closed and all current checklist items have verdicts. New findings need a reasoned acceptance or a fix.

## Example

```console
$ xpd opt-review finish --help | head -n 1
End the run: compare findings, line count and benchmark timing with the start commit. Exit 1 while a fix is open or a new finding has no reason
```
