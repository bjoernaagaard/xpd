Validate cardinality when a pandas merge names its left and right keys separately.

```diff lang="python"
-rows = left.merge(right, left_on='customer', right_on='id')
+rows = left.merge(right, left_on='customer', right_on='id', validate='many_to_one')
```
