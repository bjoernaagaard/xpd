A Polars join allows duplicate keys on either side by default. Pass validate when the keys should be unique on one or both sides.

```diff lang="python"
-df.join(df, on="x")
+df.join(df, on="x", validate="1:1")
```

## What the rule reports

Two identical keys on each side yielded 4 rows. 1:1 validation raised ComputeError.

## How to fix it

Edit the code using the replacement in the example.
