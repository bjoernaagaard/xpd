Validate cardinality when a Polars join uses separate or computed keys. If the join cannot validate expressions, materialize and validate the keys first.

```diff lang="python"
-rows = left.join(right, left_on='customer', right_on='id')
+rows = left.join(right, left_on='customer', right_on='id', validate='m:1')
```
