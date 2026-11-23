`groupby(...).apply` with a lambda runs a Python function once for every group. Use `agg` or `transform` with named methods, `idxmax` for the row with the largest value, or sums of products for weighted means.

```diff lang="python"
-totals = df.groupby("region")["load"].apply(lambda group: group.sum())
+totals = df.groupby("region")["load"].sum()
```

## What the rule reports

The rule reports any `groupby(...).apply` with a lambda, which runs Python once per group. The rule leaves alone a lambda that sorts each group, or that builds a dict, a list or a joined text per group.

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites single-reduction lambdas (`lambda g: g.sum()` and `lambda g: g["a"].sum()`) to the named groupby method (`g.sum()` or `g["a"].sum()`). Other lambdas remain manual.

The named reduction can change the name of the index, the dtype and the floating point rounding, so check the result before you replace `apply`.
