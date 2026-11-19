Formatting dates as text and parsing them again to drop the time was 18 to 202 times slower. Call `dt.normalize()` instead.

```diff lang="python"
-day = pd.to_datetime(df["time"].dt.strftime("%Y-%m-%d"))
+day = df["time"].dt.normalize()
```

## What the rule reports

The rule reports `pd.to_datetime(df["d"].dt.strftime("%Y-%m-%d"))` and `df["d"].dt.strftime("%Y-%m-%d") == "2023-06-01"`, which turn dates into text to drop the time and were 18 to 202 times slower than `normalize`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["d"].dt.normalize()`, `df["d"].dt.floor("h")`, and `df["d"].dt.normalize() == pd.Timestamp(day)` for the comparison, which raises TypeError on a column with a time zone.

The fix is unsafe, because on dates with a time zone the round trip through text drops the zone.
