Polars returns null for unmatched rows when then has no otherwise. Supply otherwise if those rows need a value.

```diff lang="python"
-df.select(pl.when(expr > 1).then(expr))
+df.select(pl.when(expr > 1).then(expr).otherwise(0))
```

## What the rule reports

The unhandled row held None. The explicit branch returned 0.

## How to fix it

Edit the code using the replacement in the example.
