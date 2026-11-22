Collect builds a DataFrame before writing. Use a matching sink when only the file is needed.

```diff lang="python"
-lf.collect().write_parquet("data.parquet")
+lf.sink_parquet("data.parquet")
```

## What the rule reports

Eager write and streaming sink produced equal data after reading their files.

## How to fix it

Edit the code using the replacement in the example.
