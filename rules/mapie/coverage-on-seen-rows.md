Measuring coverage on rows the model was fitted or conformalized on makes the intervals look better than they are. Measure coverage on held-out rows.

```diff lang="python"
 cross.fit_conformalize(X_train, y_train)
-y_pred, y_pis = cross.predict_interval(X_train)
-coverage = regression_coverage_score(y_train, y_pis)[0]
+y_pred, y_pis = cross.predict_interval(X_test)
+coverage = regression_coverage_score(y_test, y_pis)[0]
```

## What the rule reports

The rule reports `predict_interval` and `predict_set` on the variable that an earlier statement gave
to `fit`, `fit_conformalize` or `conformalize` of the same object, where the result goes into a
MAPIE coverage metric.

The rule reports only where the result reaches a coverage metric. The result reaches one when the
call, or the second name the call is assigned to, is an argument of `regression_coverage_score`,
`classification_coverage_score`, the size-stratified coverage (SSC) functions,
`regression_mwi_score` or `hsic`. A prediction for seen
rows that is only plotted is left alone, and so is one for a variable that a statement in between
assigns other rows to.

## How to fix it

The severity is error for rows given to `fit` or `fit_conformalize`, and warning for rows given to `conformalize`. xpd has no automatic fix. The rows held out from the fit are not in the file. xpd has no automatic fix, so make the change by hand.

| Object | Rows given to | Coverage of the same rows | Coverage of the test rows |
| --- | --- | --- | --- |
| `CrossConformalRegressor` | `fit_conformalize` | 1.000 | 0.913 |
| `SplitConformalRegressor` | `fit` | 1.000 | 0.907 |
| any | `conformalize` | 0.905, between 0.900 and 0.905 in each of the 30 runs | not given |

`conformalize` picks the quantile that covers its rows, so the 0.905, with a standard error of
0.000, shows nothing about new rows. The `conformalize` document is therefore a warning.
