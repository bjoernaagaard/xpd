`reset_index(drop=True)` after a sort renumbers the rows in a second step. Pass `ignore_index=True` to the sort instead.

```diff lang="python"
-ordered = df.sort_values("load").reset_index(drop=True)
+ordered = df.sort_values("load", ignore_index=True)
```

## What the rule reports

The rule reports `df.sort_values("b").reset_index(drop=True)`, also after `sort_index`, `dropna`, `drop_duplicates`, `explode` and `sample`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.sort_values("b", ignore_index=True)`. `reset_index` always gives a RangeIndex, and `ignore_index=True` keeps the old index when the method has nothing to do, as on an empty frame.
