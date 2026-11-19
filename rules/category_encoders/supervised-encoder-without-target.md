A supervised encoder learns from the target, so `fit(X)` without it raises. Pass the target as the second argument.

```diff lang="python"
-X_encoded = ce.TargetEncoder(cols=["city"]).fit_transform(X)
+X_encoded = ce.TargetEncoder(cols=["city"]).fit_transform(X, y)
```

## What the rule reports

The rule reports `fit(X)` and `fit_transform(X)` without the target on a supervised encoder.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The supervised encoders are `CatBoostEncoder`, `CountTargetEncoder`, `GLMMEncoder`,
`JamesSteinEncoder`, `LeaveOneOutEncoder`, `MEstimateEncoder`, `QuantileEncoder`,
`SummaryEncoder`, `TargetEncoder` and `WOEEncoder`.

| Call | Error |
| --- | --- |
| `fit_transform(X)`, on each of the ten | `TypeError: fit_transform() missing argument: y` |
| `fit(X)`, on nine of them | `ValueError: Supervised encoders need a target for the fitting. The target cannot be None` |
| `fit(X)`, on `SummaryEncoder`, whose `fit` has no default for `y` | `TypeError` |

A scikit-learn transformer takes `fit(X)`, so the form is easy to carry over from an
`OrdinalEncoder` or a `OneHotEncoder`. The rule reports the call on an encoder built in the same
expression, and on a name that is assigned from a supervised class in the same function and from
nothing else there.
