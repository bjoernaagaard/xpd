With `exog=None`, the instrument model has no intercept, because linearmodels adds none, so the estimates are biased. Pass a column of ones as `exog`.

```diff lang="python"
-model = IV2SLS(y, None, x, z)
+model = IV2SLS(y, sm.add_constant(controls), x, z)
```

## What the rule reports

The rule reports `None` written as the `exog` of an instrument model that has an endogenous
regressor. The model then has no intercept.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Over 2,000 draws of 500 rows from `y = 2 + x + error` with one instrument, the two forms gave the
following mean estimates:

| `exog` | Mean estimate of the coefficient on `x`, which is 1 |
| --- | --- |
| `None` | 2.002 |
| a constant column | 0.990 |

On the mroz data the coefficient on `educ` was 0.0928 without `exog` and 0.0664 with a constant and
`exper`. Data with mean zero needs no intercept, so the rule is a warning.
