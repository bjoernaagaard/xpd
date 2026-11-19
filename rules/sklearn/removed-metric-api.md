These metric names were removed from scikit-learn, so the code raises. Use the replacement that the message names, such as `get_scorer_names()` for `SCORERS`.

```diff lang="python"
-rmse = mean_squared_error(y_test, y_pred, squared=False)
-scorer = make_scorer(roc_auc_score, needs_proba=True)
+rmse = root_mean_squared_error(y_test, y_pred)
+scorer = make_scorer(roc_auc_score, response_method="predict_proba")
```

## What the rule reports

The rule reports metric APIs that scikit-learn removed:

- `SCORERS`, `plot_confusion_matrix` and the other `plot_*` functions, and
  `jaccard_similarity_score`
- `squared=` on `mean_squared_error` and `mean_squared_log_error`, and `eps=` on `log_loss`
- `squared=`, `needs_proba=` and `needs_threshold=` passed to `make_scorer`
- the `RocCurveDisplay` keywords that 1.7 replaced

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `root_mean_squared_error(...)` for `squared=False`, where the file imports that name or calls it through `metrics.`, dropping `squared=True` and `eps="auto"`, `response_method=` for `needs_proba=True` or `needs_threshold=True`, `name=` for `estimator_name=`, `y_score=` for `y_pred=`, and `curve_kwargs={...}` for matplotlib keywords on `RocCurveDisplay.from_estimator`, `from_predictions` and `plot`. The unsafe fix writes `metrics.RocCurveDisplay.from_estimator` and the matching Display class for `metrics.plot_*`, because each is another function and `plot_confusion_matrix` named its third argument `y_true`, dropping another value of `eps=`, which clipped the probabilities at another bound, and `curve_kwargs={...}` on `from_cv_results`, which never took matplotlib keywords. `SCORERS`, the imports of the `plot_*` functions, `jaccard_similarity_score`, `make_scorer` with `squared=` and `label=` have no fix.

xpd applies the replacement with `--fix --unsafe-fixes`.

Every form raises on scikit-learn 1.9.1. `SCORERS`, the `plot_*` functions and
`jaccard_similarity_score` raise ImportError, and the keywords raise TypeError. `make_scorer`
passes `squared=`, `needs_proba=` and `needs_threshold=` on to the metric, so the error comes when
the scorer runs. A search with the default `error_score` then scores every fold as nan.

The rule reports `squared=` and `eps=` only on the scikit-learn functions, imported from
`sklearn.metrics` or called as `metrics.<name>`, because torchmetrics has a `mean_squared_error`
with `squared=`. Old module paths such as `sklearn.metrics.ranking` are left to
[`removed-in-sklearn`](removed-in-sklearn.md).

The four `RocCurveDisplay` forms raised TypeError in the first run, with matplotlib 3.11.2 added
to scikit-learn 1.9.1. The saved rerun had no matplotlib, so the saved results hold no passing
proof for them. Their fixes are unsafe, because a form that raises leaves nothing to compare bit for bit.
Neither reader called one of the 13 held-back reports wrong. Recall was 74 of 74.
