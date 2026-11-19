Calling a date method inside `apply` runs Python once for every value. Call the same method on the whole column through `.dt`.

```diff lang="python"
-hours = df["time"].apply(lambda time: time.floor("h"))
+hours = df["time"].dt.floor("h")
```

## What the rule reports

The rule reports `df["d"].apply(lambda t: t.floor("h"))`, and the same with `ceil`, `normalize`, `tz_localize`, `tz_convert`, `day_name`, `month_name`, `to_period`, `total_seconds`, `weekday`, `date`, `isocalendar`, `timestamp`, `toordinal`, `replace(tzinfo=None)` and `astimezone`, which call Python once per date. It leaves `round` alone, because `lambda v: v.round(2)` is more often a number. It also leaves `replace(tzinfo=zone)` alone, because `replace` skips the daylight saving check that `tz_localize` makes.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["d"].dt.floor("h")`, `df["d"].dt.weekday`, `df["d"].dt.date`, `df["d"].dt.isocalendar().week`, `df["d"].dt.tz_localize(None)` for `replace(tzinfo=None)`, `df["d"].dt.tz_convert(zone)` for `astimezone(zone)`, and `(df["d"] - pd.Timestamp("1970-01-01")) / pd.Timedelta("1s")` for `timestamp()`, which raises TypeError on a column with a time zone. `toordinal` has no fix.

The table shows the measured speed of the column forms.

| Lambda | Column form | Rows | Speed |
| --- | --- | --- | --- |
| `t.floor("h")` | `dt.floor("h")` | 1,000,000 | 1,354 times faster |
| `t.replace(tzinfo=None)` | `dt.tz_localize(None)` | 200,000 | 2.3 ms against 746 ms |
| `t.timestamp()` | `(df["d"] - pd.Timestamp("1970-01-01")) / pd.Timedelta("1s")` | not stated | 229 times faster |

The fix is unsafe, because the dtype can change. `dt.weekday` is int32 where the lambda gave int64, and `isocalendar().week` is UInt32 where the lambda gave int64.
