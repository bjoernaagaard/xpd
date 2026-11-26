Record the plan of a review round. The file holds the ordered `"fixes"` (each naming mechanical finding ids or `"goal": "design"` with per-figure checklist `"questions"`), `"accepted"` mechanical findings with one-sentence reasons, and `"answered"` per-figure checklist items with `"verdict"` (`"pass"`, `"na"`, or `"accepted"`) and `"reason"`. xpd refuses a plan that names a finding or checklist item not present in the current ledger, or a plan while a fix is still open.

The [`xpd chart-review`](../../chart-review/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd chart-review plan --help
...
```
