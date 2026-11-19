Date arithmetic inside `apply` runs Python once for every date. Add or subtract on the whole column.

```diff lang="python"
-next_week = df["time"].apply(lambda time: time + pd.Timedelta(days=7))
+next_week = df["time"] + pd.Timedelta(days=7)
```

## What the rule reports

The rule reports `df["d"].apply(lambda t: t + pd.Timedelta(days=7))`, `t.replace(day=1)` and `t.strftime(...)`, which call Python once per date. For `replace(day=1)` the fix writes `df["d"] - pd.to_timedelta(df["d"].dt.day - 1, unit="D")`, which keeps the time of day.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["d"] + pd.Timedelta(days=7)`.

The fixes are unsafe, because a missing date gives a missing result where the lambda raised.
