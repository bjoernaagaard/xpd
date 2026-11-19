Arithmetic inside `apply` runs Python once for every value. Write the same arithmetic on the whole column.

```diff lang="python"
-scaled = df["load"].apply(lambda value: value * 2 + 1)
+scaled = df["load"] * 2 + 1
```

## What the rule reports

The rule reports `df["b"].apply(lambda v: v * 2 + 1)`, which calls Python once per value where the same arithmetic on the column runs once. The fix is unsafe, because a column method returns a missing value where the lambda raised. The dtype can also change, as when `dt.year` gives int32 where the lambda gave int64.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["b"] * 2 + 1`.
