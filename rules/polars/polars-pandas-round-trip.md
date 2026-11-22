A Polars to pandas to Polars round trip converts the data twice. Keep the original frame when no pandas operation is needed.

```diff lang="python"
-pl.from_pandas(df.to_pandas())
+df
```

## What the rule reports

The integer-only round trip matched the original frame on 50,000 and 500,000 rows.

## How to fix it

Edit the code using the replacement in the example.
