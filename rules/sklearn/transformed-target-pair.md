`TransformedTargetRegressor` raises ValueError at fit when it gets `func` without `inverse_func`, or both a `transformer` and functions. Pass the matching pair, such as `np.log1p` with `np.expm1`.

```diff lang="python"
-model = TransformedTargetRegressor(regressor=Ridge(), func=np.log1p)
+model = TransformedTargetRegressor(regressor=Ridge(), func=np.log1p, inverse_func=np.expm1)
```

## What the rule reports

The rule reports `TransformedTargetRegressor` with `func` and no `inverse_func`, with
`inverse_func` and no `func`, or with both a `transformer` and functions.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `inverse_func=np.expm1` beside `func=np.log1p`, because the old form raises at fit. The other forms have no fix.

xpd applies the replacement with `--fix --unsafe-fixes`.

Each of the three mixes raises ValueError at fit. Recall was 31 of 36, and the rule found none of 5
plants that import the class under another name, as in `TransformedTargetRegressor as TTR`.
