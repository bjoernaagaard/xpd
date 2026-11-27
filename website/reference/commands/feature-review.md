Use `xpd feature-review` to review the domain mechanism, cutoff and development-fold value of each candidate model feature.

The bare command lists findings and per-feature or per-hypothesis questions. Paths default to `.` or to those recorded by `start`. Use `--kind`, `--limit` and `--since` to compare or filter the display. `--questions` prints the catalog without a project. JSON lists `read`, `findings`, `summary` and `checklist`. Findings include their source rule and locations. `read` holds the manifest, graph and runtime facts. A static-only list has `read.facts: null`.

```console
$ xpd feature-review --questions --format json | jq -c '[.questions[].id] | sort'
["availability","lineage","mechanism","regimes","value"]
```

## Manifest

Commit `feature-review.json` at the Git root. Select every owning file. A feature needs an output `column` and can name a graph `node` that resolves once in its owner. A hypothesis can live in an existing project note. Use its original ID when implementing it.

```json title="feature-review.json"
{
  "target": "price",
  "prediction_cutoff": "11:00 UTC on the day before delivery",
  "development": {"snapshot": "training-2025", "holdout": "delivery year 2026"},
  "baseline": ["hour"],
  "inputs": {"hour": "Delivery hour", "load": "Load forecast", "wind": "Wind forecast"},
  "features": [
    {"id": "residual", "name": "Residual load", "kind": "feature",
     "owner": "features.py", "node": "residual_load", "column": "residual_load"}
  ]
}
```

A `rolling` declaration has `source` (a parameter or literal column), optional `group_keys` and optional `prior_only`. The survey checks direct trailing windows, all-row target encoding before outer cross-validation, and exact arithmetic duplicates across declared nodes in one Driver. Prior-only declarations should reflect what was available at publication time.

## Runtime adapter

A Python adapter defines `review()` and returns a mapping with these fields.

| Field | Contract |
| --- | --- |
| `development` | pandas DataFrame with a unique nonmissing index of row IDs, raw predictors and the manifest target. Include timing and group metadata when needed. Load only the development partition. |
| `splits` | Ordered pairs of training and validation row IDs. Supply at least two distinct splits with nonempty, disjoint sides. |
| `mode` | `time`, `group`, `both` or `iid`. |
| `prediction_time`, `label_available_time` | Column names for time mode. Use prediction timestamps sorted by row and split. The runner checks chronology and label publication. |
| `group`, `justification` | Group column name for group mode or a reason for IID. |
| `build_features` | Callable receiving `train_X, train_y, validation_X`. Return aligned training and validation DataFrames and a dict mapping categorical feature IDs to pairs of raw training and validation Series. |
| `make_estimator` | Callable receiving ordered column names. Return a fresh estimator with fixed settings and seeds. Put variant-specific preprocessing in this estimator. |
| `score`, `metric`, `direction` | Callable receiving the fitted estimator, validation features and validation target. Declare the metric name and direction (`loss` or `gain`). |

For supervised features, make training encodings out of fold under the inner time or group policy. Map validation using fitted training state. Any tuning or early stopping uses only the training partition.

A command adapter executes at the Git root of the measured tree and prints the same JSON as the bundled Python runner. Its `schema_version` is 1. Its `protocol` contains `development_fingerprint`, `split_fingerprint`, `development_rows`, `mode`, `baseline`, `metric`, `direction` and optional `justification`. Each ordered `folds` entry has an `id`, `train_count`, `validation_count`, `boundary`, `baseline_score`, and `stability`, `unseen`, `clusters` and `ablation` facts. The boundary declares row `overlap`, group `group_overlap` when applicable and UTC `train_latest`, `validation_cutoff` and `train_label_latest` in time mode. The Python runner checks row membership directly. A command adapter declares its provenance.

Stability reports usable count and missing share, numeric median and IQR or raw categorical level count. Unseen reports numerator, denominator, rate and missing count. Clusters report training Spearman edges with pair counts and connected components. Ablation reports baseline-plus-one score, the paired improvement, and its mean and range across folds. Undefined values have a reason. The output contains aggregate evidence without raw rows or labels.

## Recorded stages

| Stage | Purpose |
| --- | --- |
| [start](start/) | Record the committed baseline, checks and adapter. |
| `xpd feature-review` | List current findings, measurements and checklist answers. |
| [plan](plan/) | Record fixes, accepted findings and checklist verdicts. |
| [gate](gate/) | Measure one committed fix against the last kept commit. |
| [finish](finish/) | Close a run when all fixes and current questions are settled. |
| [status](status/) | Print the current run and next stage. |
| [approve](approve/) | Approve a fix that changes an interface. |
| [drop](drop/) | Record a reason for abandoning a fix. |

See the [feature review guide](../../guides/feature-review/) for the review sequence.

## Exit status

| Status | Meaning |
| --- | --- |
| `0` | The list or catalog printed, or the recorded stage succeeded. |
| `1` | The gate rejected a fix, or finish found open fixes, unexplained findings or unanswered items. |
| `2` | A manifest, adapter, protocol, plan or other command input was invalid. |
