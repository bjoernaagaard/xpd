Formatting dates as text and parsing a number back out was 33 to 198 times slower than reading the part. Read it through `.dt`, as in `df["d"].dt.year`.

```diff lang="python"
-year = df["time"].dt.strftime("%Y").astype(int)
-weekday = df["time"].dt.strftime("%A")
+year = df["time"].dt.year
+weekday = df["time"].dt.day_name()
```

## What the rule reports

The rule reports `df["d"].dt.strftime("%Y").astype(int)`, the same with `%m`, `%H` and `%j`, and `dt.strftime("%A")` and `"%B"`, which were 33 to 198 times slower than the date part on 1,000,000 rows.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["d"].dt.year`, `df["d"].dt.day_name()`.

The fixes are unsafe, because a date part is int32 where `astype(int)` gave int64, and `day_name` is English where `strftime` follows the locale.
