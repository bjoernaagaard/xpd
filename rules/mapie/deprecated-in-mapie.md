MAPIE renamed or stopped reading these forms, and they warn. Use the replacement that the message names.

```diff lang="python"
-risk = BinaryClassificationRisk(risk_occurrence=missed, risk_condition=positive, higher_is_better=False)
-y_pred, y_pis = cross.predict_interval(X_test, aggregate_predictions="median")
-series.update(X_new, y_new, gamma=0.05)
+risk = BinaryRisk(risk_occurrence=missed, risk_condition=positive, higher_is_better=False)
+y_pred, y_pis = cross.predict_interval(X_test, aggregate_point_predictions="median")
+series.update(X_new, y_new)
+series.adapt_conformal_inference(X_new, y_new, gamma=0.05)
```

## What the rule reports

The rule reports forms that MAPIE renamed or stopped reading:

| Old form | Since | Write instead |
| --- | --- | --- |
| `BinaryClassificationRisk` | 1.4 | `BinaryRisk` |
| `aggregate_predictions=` on `CrossConformalRegressor.predict` and `predict_interval` | 1.4.1 | `aggregate_point_predictions=` |
| `ensemble=` on the same methods of `JackknifeAfterBootstrapRegressor` | 1.4.1 | `aggregate_point_predictions=` |
| `gamma=`, `confidence_level=` and `optimize_beta=` on `TimeSeriesRegressor.update`, which reads none of them | 1.3 | `update` without them, and `adapt_conformal_inference` for `gamma` |

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `BinaryRisk` for `BinaryClassificationRisk`, and `aggregate_point_predictions=` for `aggregate_predictions=` and for `ensemble=` on a `JackknifeAfterBootstrapRegressor`. `update` has no fix, because the call of `adapt_conformal_inference` is a new statement.

The fixes are safe. A controller built with `BinaryRisk` picked the same thresholds and gave the
same predictions as one built with `BinaryClassificationRisk`, and the intervals with the new
keywords were bit equal to the ones with the old keywords. A file that imports both risk classes
ends with `BinaryRisk` twice in its import.

`ensemble=` is reported only on an object that the file assigns from
`JackknifeAfterBootstrapRegressor` and from no other class, because `TimeSeriesRegressor.predict`
still takes it. With `method="aci"`, the intervals after
`update(X, y, confidence_level=0.9, gamma=0.5)` were bit equal to the ones after
`update(X, y, confidence_level=0.9)` on MAPIE 1.3.0, 1.4.0 and 1.6.0. They differed on MAPIE
1.2.0, which needs `confidence_level` in `update`, so the `update` document carries `since: "1.3"`.
