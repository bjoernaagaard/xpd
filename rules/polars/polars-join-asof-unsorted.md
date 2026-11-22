An as-of join needs ascending join keys. Check that each input is sorted by the join key within each group.

```diff lang="python"
-df.join_asof(other, on="x", check_sortedness=False)
+df.sort("x").join_asof(other.sort("x"), on="x")
```

## What the rule reports

Descending right keys raised InvalidOperationError with the default sortedness check.

## How to fix it

Edit the code using the replacement in the example.
