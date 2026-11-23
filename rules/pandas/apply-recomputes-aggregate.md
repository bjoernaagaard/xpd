A lambda that computes `df["b"].mean()` runs that mean again for every row, so the time grows with the square of the row count. Compute the value once and use it on the whole column.

```diff lang="python"
-centered = df["load"].apply(lambda value: value - df["load"].mean())
+centered = df["load"] - df["load"].mean()
```

## What the rule reports

The rule reports `df["b"].apply(lambda v: v - df["b"].mean())`, which computes the mean of the whole column again for every row. The time grows with the square of the row count. On 20,000 rows the lambda took 0.34 seconds and the column form took 0.2 milliseconds. The rule reports a reduction of a column taken by a text label. It leaves `lambda v: v["scores"].mean()` alone, because the lambda reads the row's own value.

## How to fix it

`xpd check --fix --unsafe-fixes` replaces the lambda parameter with the column expression in arithmetic and comparison bodies, computing the aggregate once on the whole column. Conditional and multi-step lambdas remain manual.
