Use `xpd feature-review gate` in the [recorded feature review](../../feature-review/#recorded-stages).

Commit the fix and pass its ID. A mechanical fix removes a cited finding without increasing the finding count. A design fix changes a file that owns a cited item. Both rerun passing checks and compare compatible fold protocols. Kept edits can reopen answers.

## Example

```console
$ xpd feature-review gate --help
Gate a fix that is committed: run the checks, time `--bench` when set, count lines and compare the findings with the commit before it. Exit 1 when the fix is not kept
...
```
