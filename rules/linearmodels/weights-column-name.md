In linearmodels, `weights` takes the values and not the name of a column, so a name raises. Pass the column itself, as in `weights=df["w"]`.

```diff lang="python"
-model = PooledOLS.from_formula("lwage ~ 1 + exper", data, weights="hours")
+model = PooledOLS.from_formula("lwage ~ 1 + exper", data, weights=data["hours"])
```

## What the rule reports

The rule reports `weights="name"` on a model or on its `from_formula`. statsmodels and R take the
weights of a formula model as the name of a column, but linearmodels takes the values.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `weights=data["hours"]` for `weights="hours"` in `from_formula` when the data is a name. The call raised before the fix. A model built from arrays has no fix, because the frame is not in the call.

7.1 raised `TypeError: weights must be array_like ... Got str` on `PooledOLS` and on `IV2SLS`, and 7.0 raised `AttributeError: 'str' object has no attribute 'ndim'`.
