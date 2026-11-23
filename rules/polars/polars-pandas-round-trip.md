A Polars to pandas to Polars round trip converts the data twice. Keep the original frame when no pandas operation is needed.

```diff lang="python"
-pl.from_pandas(df.to_pandas())
+df
```

## What the rule reports

The integer-only round trip matched the original frame on 50,000 and 500,000 rows.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to remove the `pl.from_pandas(df.to_pandas())` round trip.
