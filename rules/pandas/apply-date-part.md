Reading a date part such as the year inside `apply` runs Python once for every value. Read it from the whole column through `.dt`.

```diff lang="python"
-years = df["time"].apply(lambda time: time.year)
+years = df["time"].dt.year
```

## What the rule reports

The rule reports `df["d"].apply(lambda t: t.year)`, which calls Python once per value where the `dt` accessor works on the whole column. The fix is unsafe, because a column method returns a missing value where the lambda raised. The dtype can also change, as when `dt.year` gives int32 where the lambda gave int64.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["d"].dt.year`.
