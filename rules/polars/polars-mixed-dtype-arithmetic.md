Polars 2.1.0 rejects string and numeric Series arithmetic. Cast the strings explicitly, or use string concatenation for text.

```diff lang="python"
-pl.Series(["1"]) + pl.Series([2])
+pl.Series(["1"]).cast(pl.Int64) + pl.Series([2])
```

## What the rule reports

InvalidOperationError for two Series. A string Series plus integer scalar raised TypeError.

## How to fix it

Edit the code using the replacement in the example.
