A conformalized quantile regressor needs a model with the quantile loss, and fit raises without it. Pass `loss="quantile"` to the model.

```diff lang="python"
-mapie = ConformalizedQuantileRegressor(GradientBoostingRegressor(random_state=0), confidence_level=0.9)
+mapie = ConformalizedQuantileRegressor(GradientBoostingRegressor(loss="quantile", random_state=0), confidence_level=0.9)
```

## What the rule reports

The rule reports `ConformalizedQuantileRegressor` and `CrossConformalizedQuantileRegressor` with an
estimator built in the argument list that has no quantile loss. Such an estimator is
`GradientBoostingRegressor` or `HistGradientBoostingRegressor` without `loss="quantile"`,
`LGBMRegressor` without `objective="quantile"`, or a scikit-learn regressor that has none, such as
`RandomForestRegressor`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The fix is unsafe even though the old fit raises. It writes `loss="quantile"` in place of the loss of `GradientBoostingRegressor` and `HistGradientBoostingRegressor`, at the end of the arguments, which drops the comments between them. `LGBMRegressor`, which needs an alpha, a call with a positional argument and another model have no fix.

| Estimator | Result |
| --- | --- |
| a boosting model without the quantile loss | "You need to set the loss/objective argument of your base model to `quantile`" |
| a model of another class | "The base model is not supported" |
| `GradientBoostingRegressor(loss="quantile")`, `HistGradientBoostingRegressor(loss="quantile")` | runs, because MAPIE sets the quantile itself |
| `LGBMRegressor(objective="quantile")` | "The matching parameter `alpha_name` for estimator does not exist", until it also gets an `alpha` |

The rule reads only an estimator built in the argument list. A `loss` or `objective` that isn't
written as text, such as `loss=loss`, may hold `"quantile"` and is left alone.
