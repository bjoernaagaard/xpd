With `LeaveOneOut` each fold tests one row, and scores such as R2 or ROC AUC need more than one row, so every fold scores nan. Score once on `cross_val_predict(..., cv=LeaveOneOut())`, or use `KFold`.

```diff lang="python"
-scores = cross_val_score(Ridge(), X, y, cv=LeaveOneOut())
+r2 = r2_score(y, cross_val_predict(Ridge(), X, y, cv=LeaveOneOut()))
```

## What the rule reports

The rule reports `cv=LeaveOneOut()` with a score that needs more than one row. Such a score is R2
or ROC AUC given as `scoring`, or the R2 that a regressor scores by default.

## How to fix it

xpd has no automatic fix, so make the change by hand.

With one row per fold, each fold warns and scores nan, and a search over nan scores picks its first
candidate. Under `python -W error` each warning is an `UndefinedMetricWarning` that raises.

| Scoring | Warning on a fold of one row |
| --- | --- |
| R2 | "R^2 score is not well-defined with less than two samples" |
| ROC AUC | "Only one class is present in y_true" |

The rule covers `cross_val_score`, `cross_validate`, `GridSearchCV`, `RandomizedSearchCV`,
`validation_curve`, `learning_curve`, `permutation_test_score` and `RFECV`. It knows a regressor
only by its class name in the same call. `explained_variance` and `average_precision` score 1 or 0
on a fold of one row and give no nan, so they are left alone. `cv=len(X)`, which is leave one out
under another name, is left alone by design.

Recall was 14 of 33, and the plants included the two scorings that the rule left out after the
recall run. The rule misses a splitter kept in a name, as in `loo = LeaveOneOut()` and `cv=loo`,
and `LeaveOneOut` imported under another name. It also misses a regressor passed by a variable
with no `scoring`.
