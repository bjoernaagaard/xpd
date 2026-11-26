Gate the fix that is open, after it is committed. xpd runs each check that passed at the start, runs the recorded adapter when set, and compares the mechanical findings and checklist with the commit before the fix. It keeps the fix when each check passes, the count of mechanical findings is not higher, and either a named mechanical finding is gone or a `"goal": "design"` fix modified a file owning its named checklist items (marking those items fixed automatically). It exits 1 when the fix is not kept and records the reason.

The [`xpd chart-review`](../../chart-review/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd chart-review gate --help
...
```
