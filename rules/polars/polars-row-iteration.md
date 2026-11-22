Use a Polars expression to compute a reduction over rows. A column sum ran 1,279 and 623 times faster on 50,000 and 500,000 rows.

```diff lang="python"
-sum(row[0] for row in df.iter_rows())
+df["x"].sum()
```

## What the rule reports

1279.48x and 623.31x with equal sums on integer data.

## How to fix it

Edit the code using the replacement in the example.
