A join to an aggregation of the same Polars frame can often be written as a window expression. Check the output names and how null keys should be grouped.

```diff lang="python"
-df.join(df.group_by("y").agg(total=expr.sum()), on="y")
+df.with_columns(total=expr.sum().over("y"))
```

## What the rule reports

Three rows with non-null keys gave equal frames.

## How to fix it

Edit the code using the replacement in the example.
