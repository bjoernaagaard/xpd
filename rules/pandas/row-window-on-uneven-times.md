A rolling window given as a number counts rows, even with `on=` naming a time column, so on days with gaps it reaches further back than you meant. Pass the span as an offset, as in `rolling("7D", on="day")`.

```diff lang="python"
-weekly = df.rolling(7, on="day")["sold"].sum()
+weekly = df.rolling("7D", on="day")["sold"].sum()
```

## What the rule reports

The rule reports `rolling` with an integer window and `on=`. A window given as a number counts rows, also when `on=` names a time column. `rolling(3)` without `on=` gave the same numbers as with it. The table shows the sum for 2024-01-10 on six days with gaps.

| Call | Sum for 2024-01-10 | Rows in the window |
| --- | --- | --- |
| `rolling(3, on="day").sum()` | 14.0 | three rows that reach back 8 days |
| `rolling("3D", on="day").sum()` | 8.0 | the one row of the last 3 days |

The rule misses a row window on a frame with the times in its index, because xpd can't see what the index holds.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A time window starts with `min_periods=1`, where the row window gives missing values until it has 7 rows.
