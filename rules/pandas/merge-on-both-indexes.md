A merge on both indexes is a join, but the merge call hides that rows are paired by index label. Call `join` with the same `how` instead.

```diff lang="python"
-both = left.merge(right, left_index=True, right_index=True)
+both = left.join(right, how="inner")
```

## What the rule reports

The rule reports `left.merge(right, left_index=True, right_index=True)` and the `pd.merge` form. The merge gave a frame equal to `left.join(right, how="inner")`, and the two were also equal for `how="left"`, `"right"` and `"outer"` and with `validate="1:1"`. The fix is unsafe, because `join` raises ValueError when the two frames share a column name. `merge` adds the suffixes `_x` and `_y` to such columns.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `left.join(right, how="inner")`. None for a merge with another keyword than `how`, `validate` and `sort`.
