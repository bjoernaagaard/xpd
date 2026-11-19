Reading a date part inside `apply` and comparing it runs Python once for every value, and was 22 to 94 times slower. Compare the whole column through `.dt`, as in `df["d"].dt.weekday >= 5`.

```diff lang="python"
-weekend = df["time"].apply(lambda time: time.weekday() >= 5)
+weekend = df["time"].dt.weekday >= 5
```

## What the rule reports

The rule reports `df["d"].apply(lambda t: t.weekday() >= 5)` and `lambda t: t.hour * 60 + t.minute`, a lambda that compares a date part or computes with parts, which was 22 to 94 times slower than the column form. The rule has no fix, because the rewrite depends on the operator.

## How to fix it

xpd has no automatic fix, so make the change by hand.
