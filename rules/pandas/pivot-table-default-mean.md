`pivot_table` without `aggfunc` takes the mean of the rows that share a cell and says nothing, so two rows of 10 and 30 become 20.0. Name the aggregation, or call `pivot`, which raises when a cell has more than one row.

```diff lang="python"
-table = df.pivot_table(index="city", columns="year", values="sold")
+table = df.pivot_table(index="city", columns="year", values="sold", aggfunc="sum")
```

## What the rule reports

The rule reports `pivot_table` without `aggfunc`, which takes the mean of the rows that share a cell without a warning. The table shows the cell for two rows of 10 and 30 with one city and one year.

| Call | Cell |
| --- | --- |
| `pivot_table` without `aggfunc` | 20.0 |
| `pivot_table(aggfunc="sum")` | 40 |
| `pivot` | ValueError "Index contains duplicate entries, cannot reshape" |

## How to fix it

xpd has no automatic fix, so make the change by hand.

Write `aggfunc="mean"` when you want the mean.
