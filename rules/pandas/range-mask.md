Two comparisons joined with `&` state one range twice. Call `between(low, high)` instead.

```diff lang="python"
-inside = (df["load"] >= 10) & (df["load"] <= 20)
+inside = df["load"].between(10, 20)
```

## What the rule reports

The rule reports `(df["a"] >= 10) & (df["a"] <= 20)`. The fix is unsafe, because the value can be a NumPy array, which has no `between`.

The rule leaves comparisons alone on `df.index`, `df.index.hour`, `df.columns`,
`df["load"].values` and `df["load"].array`, because an index or an array has no `between`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["a"].between(10, 20)`.
