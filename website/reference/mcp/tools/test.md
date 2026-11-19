Use `test` while you write project rules, as [`xpd test`](../../../commands/test/) does. It checks
the valid and invalid examples of each rule with the same scanner as `check`, and compares the
snapshots. It changes snapshot files only with `update_snapshots`.

## Example

```json
{"options":{"filter":"^team/","update_snapshots":true}}
```

The result has `cases`, each with its `id`, `source` and `status`, and the `failed` count.

## Errors

When no test case matches, the call fails. A rule that `new` just made has an empty case, so it
fails this way until the case has examples.

```text
xpd test --skip-snapshot-tests -- exited with 2: xpd: no cases matched
```

`skip_snapshots` and `update_snapshots` together fail with "Choose either skipping or updating
snapshots."
