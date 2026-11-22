Polars 2.0 made streaming the default for lazy queries. Sort on the keys that define the required order before taking a positional slice.

```diff lang="python"
-lf.unpivot(on=["x", "y"]).head(2).collect()
+lf.unpivot(on=["x", "y"]).sort("variable", "value").head(2).collect()
```

## What the rule reports

The first two unpivoted rows had labels b and c with streaming and b and b with the in-memory engine.

## How to fix it

Edit the code using the replacement in the example.
