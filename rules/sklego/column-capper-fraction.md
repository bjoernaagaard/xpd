`ColumnCapper` reads `quantile_range` in percent, so `(0.05, 0.95)` caps almost every value at a percentile below 1. Multiply both numbers by 100.

```diff lang="python"
-capper = ColumnCapper(quantile_range=(0.05, 0.95))
+capper = ColumnCapper(quantile_range=(5, 95))
```

## What the rule reports

The rule reports `ColumnCapper(quantile_range=(0.05, 0.95))` and other ranges whose upper number is
1 or less. `ColumnCapper` reads the range in percent, as its default `(5.0, 95.0)` shows, so
fractions cap every value at a percentile below 1.

## How to fix it

xpd has no automatic fix, so make the change by hand.

| `quantile_range` | Values changed, of 10,000 normal values | Values after capping |
| --- | --- | --- |
| `(0.05, 0.95)` | 9,910 | 92 distinct values from -3.272 to -2.379 |
| `(5, 95)` | 1,000 | from -1.628 to 1.640 |
