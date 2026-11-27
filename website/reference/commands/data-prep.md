Use `xpd data-prep` to check Python preparation code from raw data through imputation, splitting, and scaling the features and target. The [data preparation guide](../../guides/data-prep/) shows how to record the work.

The bare command lists findings, the pipeline graph, and optional facts and checklist items for the columns. Findings are grouped by leakage, joins, target transforms, coercion, and pipeline shape. Use `--format json` to get their stable IDs, and `--questions` to list the catalog without scanning files.

## Adapters

Pass `--adapter prep.py` when a Python file defines `columns()` returning a pandas or Polars frame, a path, or a dictionary of lists. xpd runs it in the project's Python with pandas and SciPy. A shell command adapter prints JSON with a `columns` array of measured column objects. It may supply `rows`, `target` and `model_family`. Each column object has a `column` name and may supply `min`, `skew`, `outlier_share` and `zero_share` for scaling advice.

`--target <column>` overrides the adapter's target name. Otherwise xpd uses the adapter's target or infers it from a column named `y`, `target` or `label`. A survey with no named target still gets feature questions.

## A recorded run

| Stage | Command |
| --- | --- |
| Start | `xpd data-prep start <paths> --check "<command>" --adapter prep.py` |
| List | `xpd data-prep <paths> --adapter prep.py --format json` |
| Plan | `xpd data-prep plan plan.json` |
| Approve an interface change | `xpd data-prep approve <fix-id>` |
| Gate a committed fix | `xpd data-prep gate <fix-id>` |
| Drop a restored fix | `xpd data-prep drop <fix-id> --reason "<reason>"` |
| Resume | `xpd data-prep status` |
| Finish | `xpd data-prep finish` |

xpd stores runs under `.git/xpd-data-prep.json`, or in the shared `.git/xpd-ledger.sqlite` in durable mode. Finish exits 1 when a fix is open, a new finding needs a reason, or a checklist item is unanswered.

```console
$ xpd data-prep --questions --format json | jq -c '{facts: (.facts | length), questions: (.questions | length)}'
{"facts":14,"questions":13}
```

## Exit status

| Code | Meaning |
| --- | --- |
| `0` | The requested list or run stage succeeded. |
| `1` | The gate rejected the fix or the run has unresolved work. |
| `2` | The command could not run, for example because the arguments or recorded adapter were invalid. |
