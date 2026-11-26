End the review run. xpd compares the mechanical findings with the commit the run started from, checks that every per-figure checklist item is either fixed or answered in a plan, and prints the summary. It exits 1 while a fix is open, while a new finding since the start has no accepted reason, or while any per-figure checklist item remains unanswered.

The [`xpd chart-review`](../../chart-review/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd chart-review finish --help
...
```
