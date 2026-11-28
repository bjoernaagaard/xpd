Verify a saved scikit-learn pipeline contract and its scripts (`profile`, `check`,
`graph`, `refit`, and `equiv`) inside a durable local workflow. Each run stores
its state in SQLite under the user state directory and caches activity outputs by
input hash so retries skip unchanged stages.

## Examples

```console
$ xpd gates --help
...
```

## Exit status

Exits `0` when all gates pass (and for `status`), `1` when one or more gates fail,
and `2` when the contract is invalid, a file is missing, another command holds the
run lock, or an activity errors.
