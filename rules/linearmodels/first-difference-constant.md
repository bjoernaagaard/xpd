In linearmodels, `FirstDifferenceOLS` takes no constant, so a `1` term or `add_constant` raises ValueError. Take the constant out, because the difference removes the intercept.

```diff lang="python"
-model = FirstDifferenceOLS.from_formula("lwage ~ 1 + hours", data)
-model = FirstDifferenceOLS(y, sm.add_constant(X))
+model = FirstDifferenceOLS.from_formula("lwage ~ hours", data)
+model = FirstDifferenceOLS(y, X)
```

## What the rule reports

The rule reports a `1` term in a formula of `FirstDifferenceOLS`, and `add_constant(...)` as its
`exog`. A first difference removes everything that is constant over time, so the model refuses a
constant.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

With the unsafe fix, the `1` term is taken out of the formula, and `add_constant(X)` becomes `X`. The call raised before the fix. A formula whose only term is the `1` has no fix.

Both forms raised `ValueError: Constants are not allowed in first difference regressions.` on 7.1 and 7.0.
