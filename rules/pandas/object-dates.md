`dt.date` and `dt.time` turn a datetime column into a column of Python objects, which is slow and loses the datetime methods. Use `dt.normalize()` or `dt.floor("D")` to keep datetime64.

```diff lang="python"
-days = df["time"].dt.date
+days = df["time"].dt.normalize()
```

## What the rule reports

The rule reports `df["d"].dt.date` and `dt.time`, which turn a datetime64 column into a column of Python objects. The rule leaves alone a `dt.date` that is one side of a comparison, because the replacement `dt.normalize()` keeps datetime64. A datetime64 value compared with a Python `date` raises TypeError.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`normalize` returns timestamps at midnight and not Python dates, so keep `dt.date` when another library needs Python dates. For `dt.time`, `normalize` drops the time of day, so keep the time as the time since midnight, `df["time"] - df["time"].dt.normalize()`.
