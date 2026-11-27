Run the recorded checks and compare the committed fix with its predecessor. For a design fix, cite checklist items and edit a file that owns them.

See the [recorded run](../../data-prep/#a-recorded-run) for how to use `xpd data-prep gate`.

For a fix named `split-first` with the title shown below, the run lists this line after a successful gate.

```text
    split-first  Split before fitting the scaler  (kept, 1 fixed, 0 added)
```

## Example

```console
$ xpd data-prep gate --help
Gate a fix that is committed: run the checks, time `--bench` when set, count lines and compare the findings with the commit before it. Exit 1 when the fix is not kept
...
```
