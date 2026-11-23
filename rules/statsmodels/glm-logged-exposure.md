A statsmodels GLM with a log link takes the natural log of `exposure` before adding
it to the linear predictor. Passing logged values as exposure applies the log
twice and may produce nonfinite values.

```diff
-sm.GLM(y, X, family=sm.families.Poisson(), exposure=np.log(time_at_risk))
+sm.GLM(y, X, family=sm.families.Poisson(), exposure=time_at_risk)
```

## What the rule reports

The rule checks a direct NumPy or math `log` call passed as `exposure` to
`GLM`, `GLM.from_formula`, or the formula API's `glm` when the log import
is established.

## How to fix it

Pass positive exposure on the original scale, or supply the logged values as
`offset`. If the call already has an offset, account for both contributions
before editing. In a Poisson fit on 0.15.0, raw exposure and a logged offset
produced identical coefficients. Logged exposure changed both coefficients.

[GLM exposure and offset](https://www.statsmodels.org/stable/generated/statsmodels.genmod.generalized_linear_model.GLM.html)
