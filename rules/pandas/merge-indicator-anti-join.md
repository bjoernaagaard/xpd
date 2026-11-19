Filtering the `_merge` column after `merge(indicator=True)` builds an anti join by hand. On pandas 3, use `how="left_anti"` or `how="right_anti"` instead.

```diff lang="python"
-merged = left.merge(right, on="key", how="left", indicator=True)
-unmatched = merged[merged["_merge"] == "left_only"]
+unmatched = left.merge(right, on="key", how="left_anti")
```

## What the rule reports

The rule reports `m[m["_merge"] == "left_only"]` after `merge(..., indicator=True)`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The anti join returns the rows, columns and row labels of the filtered merge without the `_merge` column, so the columns of the right frame hold missing values.
