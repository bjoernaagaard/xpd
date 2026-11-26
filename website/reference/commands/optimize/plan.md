Record the plan of a round. The file holds the fixes in the order to make them, each with the ids of the findings it removes (or a `"goal"` of `"shorter"`, `"faster"` or `"both"` for a judgment-based optimization), and the findings that are right as they are, each with a reason. xpd refuses a plan that names a finding the ledger of today does not have, a fix with neither a finding nor a goal, a speed goal without a benchmark recorded at `start`, and a plan while a fix is still open.

The [`xpd optimize`](../../optimize/#a-recorded-run) page shows a whole run.

## Example

```console
$ xpd optimize plan --help
...
```
