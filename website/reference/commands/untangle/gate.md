Gate the fix that is open, after it is committed. xpd runs each check that passed at the start and compares the findings with the commit before the fix. It keeps the fix when each check passes, at least one finding of the fix is gone, and the count of findings is not higher. It exits 1 when the fix is not kept and records the reason, and the same fix can be gated again after a repair.

The [`xpd untangle`](../../untangle/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd untangle gate --help
...
```
