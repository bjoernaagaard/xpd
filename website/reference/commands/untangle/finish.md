End the run. xpd compares the findings with the commit the run started from, and prints how many were fixed, added and left without a reason. It exits 1 while a fix is open, or while a finding that is new since the start has no reason.

The [`xpd untangle`](../../untangle/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd untangle finish --help
...
```
