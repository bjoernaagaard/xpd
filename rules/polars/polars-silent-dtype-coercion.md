Use a strict Polars cast so invalid values raise instead of becoming nulls.

```diff lang="python"
-frame = frame.with_columns(pl.col('amount').cast(pl.Float64, strict=False))
+frame = frame.with_columns(pl.col('amount').cast(pl.Float64))
```
