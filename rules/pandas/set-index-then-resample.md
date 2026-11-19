Setting a column as the index only to `resample` takes an extra step. Pass the column to `resample` with `on=`.

```diff lang="python"
-daily = df.set_index("time").resample("D")["load"].sum()
+daily = df.resample("D", on="time")["load"].sum()
```

## What the rule reports

The rule reports `df.set_index("d").resample("D").sum()`. The rule reports only a chain that ends in a named aggregation such as `sum` or `mean`, because `asfreq` raises with `on=`. A function passed to `agg` also sees other data with `on=`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.resample("D", on="d").sum()`, which raises on an empty frame where the indexed form returned.
