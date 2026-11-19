`get_level_values(level).unique()` builds one value for every row before it finds the unique ones. Call `index.unique(level)`, which was 22 times faster.

```diff lang="python"
-regions = df.index.get_level_values("region").unique()
+regions = df.index.unique("region")
```

## What the rule reports

The rule reports `df.index.get_level_values("k").unique()` and `.nunique()`, which build one value per row first. On 1,000,000 rows `unique` with a level ran 22 times faster. The rule doesn't run on test files.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.index.unique("k")`, `df.index.unique("k").nunique()`.

The fix is unsafe, because a level built from a range comes back as a RangeIndex, where the old form gave an Index. The values are the same.
