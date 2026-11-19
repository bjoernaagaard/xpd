`groupby(...).apply` with a lambda runs a Python function once for every group. Use `agg` or `transform` with named methods, `idxmax` for the row with the largest value, or sums of products for weighted means.

```diff lang="python"
-totals = df.groupby("region")["load"].apply(lambda group: group.sum())
+totals = df.groupby("region")["load"].sum()
```

## What the rule reports

The rule reports any `groupby(...).apply` with a lambda, which runs Python once per group. The rule leaves alone a lambda that sorts each group, or that builds a dict, a list or a joined text per group.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The named reduction can change the name of the index, the dtype and the floating point rounding, so check the result before you replace `apply`.
