Statsmodels OLS warns and ignores `weights`. The fitted coefficients are those
of an unweighted regression even when the weights vary across observations.

```diff
-sm.OLS(y, X, weights=weights).fit()
+sm.WLS(y, X, weights=weights).fit()
```

## What the rule reports

The rule checks an explicit `weights` keyword on `OLS`, `OLS.from_formula`,
and the formula API's `ols`.

## How to fix it

Use `WLS` or the formula API's `wls` if the weights are proportional to inverse
observation variances. Drop the keyword if the regression should be unweighted.
In a 0.15.0 fit with nonconstant weights, the OLS slope stayed at 1.04345 with
or without the keyword and changed to 0.97627 with WLS.

[OLS and WLS implementation](https://www.statsmodels.org/stable/_modules/statsmodels/regression/linear_model.html)
