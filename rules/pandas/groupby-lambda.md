A lambda or a NumPy function in `agg` runs Python once for every group. Pass the name of the reduction, such as `"max"` or `"sum"`, instead.

```diff lang="python"
-maximum = df.groupby("region")["load"].agg(lambda group: group.max())
+maximum = df.groupby("region")["load"].agg("max")
```

## What the rule reports

The rule reports `g["b"].agg(lambda v: v.max())`, `g.agg(np.sum)`, `pivot_table(aggfunc=np.mean)`, `g.agg(lambda v: v.quantile(0.9))`, `g.agg(lambda v: (v > 0).sum())` and a lambda or NumPy function on `resample`, which run Python once per group. The rule leaves a lambda inside a list alone, because the result column would get a new name. It leaves `np.median`, `np.std` and `np.var` alone, because they treat missing values and degrees of freedom differently from the named methods.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `"max"` for min, max, median, count and nunique, `"sum"`, `"mean"`, `g.quantile(0.9)` for a reduction with arguments, and `"sum"` or `"max"` on a resampler for a lambda and for `np.sum`, `np.mean`, `np.min` and `np.max`. The named method can give another dtype, and it raises on a column the lambda could reduce. A count of a condition, and `np.median`, `np.std` and `np.var` on a resampler, have no fix.

The table shows how much faster the replacements were. The count of a condition has no fix.

| Reported code | Write instead | Faster by |
| --- | --- | --- |
| `g.agg(lambda v: v.quantile(0.9))` | `g.quantile(0.9)` | 4.8 times with 1,000 groups, 345 times with 100,000 |
| `g.agg(lambda v: (v > 0).sum())` | `(df["b"] > 0).groupby(df["k"]).sum()` | not measured |
| a lambda on a resampler | the named method | 3.4 to 82 times |

The fixes for `sum`, `mean`, `std`, `var` and `prod` are unsafe, because pandas adds up each group with compensated summation and the lambda didn't. Compensated summation keeps track of the rounding error of each addition, and on the values 1e16, 1, 1 and -1e16 the sum changes from 0 to 2.
