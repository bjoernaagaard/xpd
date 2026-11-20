Use `test` while you write project rules, as [`xpd test`](../../../commands/test/) does. It checks
the valid and invalid examples of each rule with the same scanner as `check`, and compares the
snapshots. It changes snapshot files only with `update_snapshots`.

## Example

The commands below make a project with the rule `team/no-print`, and the test file gives the
rule one valid and one invalid case.

```console
$ xpd new project --pack team --yes --format json | jq -c .item
"project"
$ xpd new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --with-test --yes --format json | jq -c .id
"team/no-print"
```

```yaml title="rule-tests/team/no-print-test.yml"
id: team/no-print
valid:
  - "log(1)"
invalid:
  - "import team\nprint(1)"
```

The call writes the snapshot of the invalid case.

```json
{"options":{"filter":"^team/","update_snapshots":true}}
```

The result has `cases`, each with its `id`, `source` and `status`, and the `failed` count.

```json
{"cases":"...","failed":0}
```

## Errors

When no test case matches, the call fails. A rule that `new` just made has an empty case, so it
fails this way until the case has examples.

```text
xpd test --skip-snapshot-tests -- exited with 2: xpd: no cases matched
```

`skip_snapshots` and `update_snapshots` together fail with "Choose either skipping or updating
snapshots."
