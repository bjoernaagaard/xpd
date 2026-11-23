A condition inside `apply` runs Python once for every value. Use `np.where(condition, a, b)` on the whole column, or `clip` for a minimum or a maximum.

```diff lang="python"
-level = df["load"].apply(lambda value: "high" if value > 50 else "low")
+level = pd.Series(np.where(df["load"] > 50, "high", "low"), index=df.index)
```

## What the rule reports

The rule reports `df["a"].apply(lambda v: 1 if v > 50 else 0)` and `lambda v: min(v, 1.0)`. The rule leaves alone a lambda that reduces its argument, as in `x.isnull().all()`, because the lambda gets a whole Series per group.

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites `min`/`max` lambdas to `.clip(upper=...)`/`.clip(lower=...)`, and rewrites simple comparison conditionals to `pd.Series(np.where(...), index=...)` when `numpy` is imported as `np`. Other forms remain manual.

`pd.Series(..., index=df.index)` keeps the row labels. Check the name, the dtype and the missing values of the result.
