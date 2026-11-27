Use `xpd ts-review` to examine time series and decide how to group them before fitting a model. The bare command lists findings and, when an adapter is supplied, a checklist for each measured series and group. Use `--format json` for finding IDs and measurements, or `--since <commit>` to compare finding counts. With `--since HEAD` and no tracked edits, the comparison uses the parent of HEAD so the most recent fix is visible.

## Adapters

Pass `--adapter series.py` when a Python file has a `series()` function returning a pandas or Polars frame, a path to CSV, TSV, JSON or Parquet, or a dictionary of columns. Supply `unique_id`, `ds`, `y`, and any metadata used to describe groups. Use `{"data": frame, "periods": {"store-a": [7, 365]}, "hierarchy": {"total": ["store-a", "store-b"]}}` to declare periods in cadence steps and parent relationships. The project's Python needs NumPy, pandas, statsmodels and scikit-learn. Parquet also needs a reader such as PyArrow.

A command adapter prints the same JSON as the Python runner. Its `series` array has one object per `unique_id` with a boolean `is_time_series`, and it can supply `groups` and `clusters` arrays. The bare list reports adapter errors alongside static findings. A recorded run requires its adapter to succeed.

## A recorded run

| Stage | Command |
| --- | --- |
| Start | `xpd ts-review start <paths> --check "<command>" --adapter series.py` |
| List | `xpd ts-review <paths> --adapter series.py --format json` |
| Plan | `xpd ts-review plan plan.json` |
| Approve an interface change | `xpd ts-review approve <fix-id>` |
| Gate a committed fix | `xpd ts-review gate <fix-id>` |
| Drop an abandoned fix | `xpd ts-review drop <fix-id> --reason "<reason>"` |
| Resume | `xpd ts-review status` |
| Finish | `xpd ts-review finish` |

The journal lives in `.git/xpd-ts-review.json`, or in `.git/xpd-ledger.sqlite` in durable mode. Finish refuses an open fix, an unexplained new finding, or an unanswered item. A claim with fewer than two observed cycles needs an `na` answer explaining which claim was dropped.

```console
$ xpd ts-review --questions --format json | jq -c '{facts: (.facts | length), questions: (.questions | length)}'
{"facts":14,"questions":12}
```

## Exit status

| Code | Meaning |
| --- | --- |
| `0` | The list or recorded stage succeeded. |
| `1` | A gate rejected the fix, or finish found an open fix or unresolved checklist items or findings. |
| `2` | The stage could not run, for example because its adapter failed. |
