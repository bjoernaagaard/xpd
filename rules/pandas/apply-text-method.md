Calling a text method inside `apply` or `map` runs Python once for every value. Call the same method on the whole column through `.str`.

```diff lang="python"
-names = df["name"].map(lambda name: name.upper())
+names = df["name"].str.upper()
```

## What the rule reports

The rule reports `df["s"].map(lambda v: v.startswith("A"))`, which calls Python once per value where the `str` accessor works on the whole column. The fix is unsafe, because a column method returns a missing value where the lambda raised. The dtype can also change, as when `dt.year` gives int32 where the lambda gave int64.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["s"].str.startswith("A")`.
