Use `xpd opt-review drop` during a [recorded optimization review](../../opt-review/#recorded-stages).

Restore the run to its last kept commit, then drop the open fix with `--reason`.

## Example

```console
$ xpd opt-review drop --help | head -n 1
Record that a fix was given up, after its files were restored
```
