`confidence_level` is the share of rows to cover, which is 1 minus alpha, so `confidence_level=0.1` makes intervals that cover about a tenth of the rows. Write `confidence_level=0.9` for `alpha=0.1`.

```diff lang="python"
 alpha = 0.1
-mapie = SplitConformalRegressor(model, confidence_level=alpha)
-score = regression_mwi_score(y_test, y_pis, alpha)
+mapie = SplitConformalRegressor(model, confidence_level=1 - alpha)
+score = regression_mwi_score(y_test, y_pis, 1 - alpha)
```

## What the rule reports

The rule reports a `confidence_level` that holds the share to miss, such as
`confidence_level=0.1`, `confidence_level=[0.05, 0.1]` and `confidence_level=alpha`. It also
reports the same values given to `regression_mwi_score` and `coverage_width_based` in the place of
the level, and a percent such as `confidence_level=90`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is warning for a level below 0.5 and for a variable named `alpha` or `delta`, and error for a percent. unsafe, because the level changes to the one the code meant: `confidence_level=1 - 0.1` for `confidence_level=0.1`, 1 minus each number of a list, `1 - alpha` for the level of `regression_mwi_score` and `coverage_width_based`, and `confidence_level=0.90` for `confidence_level=90`. A variable named `alpha` or `delta` and any other percent have no fix, because the file does not show whether the variable holds a list or which share the number means.

| `confidence_level` | Share of test rows covered | Width of the intervals |
| --- | --- | --- |
| 0.1 | 0.108 | 0.292 |
| 0.9 | 0.907 | 3.609 |

MAPIE runs both levels without a warning. The rule reads a literal below 0.5 on the constructors
of the conformal classes and on `TimeSeriesRegressor.predict`, and it leaves alone 0.5 itself and a
list with a level of 0.5 or more. A risk controller is left alone, because its level is
`1 - delta` and a low one is a choice.

A percent raises "Invalid confidence_level or alpha. Allowed values are between 0 and 1", but only
from `predict_interval` or `predict_set`, after the model is fitted. A risk controller raises
"delta must be in (0, 1]".
