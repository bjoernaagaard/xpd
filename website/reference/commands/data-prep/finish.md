Close the run after resolving fixes, explaining new findings and answering all current checklist items.

See the [recorded run](../../data-prep/#a-recorded-run) for how to use `xpd data-prep finish`.

With an open fix named `split-first`, finish exits 1 and prints this diagnostic.

```text
xpd: the fix split-first is still open
```

After the fix passes its gate, any unanswered item still holds the run open.

```text
xpd: these checklist items are unanswered: amount/outlier-meaning. Address them in a fix, or answer them in a plan
```

## Example

```console
$ xpd data-prep finish --help
End the run: compare findings, line count and benchmark timing with the start commit. Exit 1 while a fix is open or a new finding has no reason
...
```
