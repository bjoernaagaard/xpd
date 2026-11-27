Use `xpd opt-review gate` during a [recorded optimization review](../../opt-review/#recorded-stages).

Commit the fix first. The gate checks removal of a cited finding, ownership for a design fix, and project checks that passed at the baseline.

## Example

```console
$ xpd opt-review gate --help | head -n 1
Gate a fix that is committed: run the checks, time `--bench` when set, count lines and compare the findings with the commit before it. Exit 1 when the fix is not kept
```
