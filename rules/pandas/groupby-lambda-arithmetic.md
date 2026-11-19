A lambda in `transform` that divides or subtracts a group total runs Python once for every group. Use `transform("sum")` or another named method and do the arithmetic on the whole column.

```diff lang="python"
-share = df.groupby("region")["load"].transform(lambda group: group / group.sum())
+share = df["load"] / df.groupby("region")["load"].transform("sum")
```

## What the rule reports

The rule reports `g["w"].transform(lambda v: v / v.sum())`, `v - v.mean()`, `v / v.max()`, the z-score, min-max scaling and `v.fillna(v.mean())`. With 1,000 groups the lambdas `v - v.mean()` and `v / v.max()` were 6 times slower than `transform` with a method name, and with 100,000 groups they were 390 times slower. Min-max scaling on a group-by that does not select one column of a frame by name has no fix.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["w"] / df.groupby("k")["w"].transform("sum")`, and `(df["w"] - g.transform("min")) / (g.transform("max") - g.transform("min"))` for min-max scaling of `g = df.groupby("k")["w"]`.

The fix is unsafe, because the sums use compensated summation, which keeps track of the rounding error of each addition. The name of the result can also change.
