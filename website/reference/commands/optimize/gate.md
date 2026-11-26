Gate the fix that is open, after it is committed. xpd runs each check that passed at the start, counts code lines, times the recorded benchmark when set, and compares the findings with the commit before the fix. It keeps the fix when each check passes, the count of findings is not higher, at least one named finding is gone (or the fix's `"goal"` is met), and any recorded benchmark did not regress by more than 15%. It exits 1 when the fix is not kept and records the reason, and the same fix can be gated again after a repair.

The [`xpd optimize`](../../optimize/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd optimize gate --help
...
```
