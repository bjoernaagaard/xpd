Polars may fail to infer the result type of a Python function. Pass return_dtype to declare the output type before collecting.

```diff lang="python"
-expr.map_elements(lambda value: (value,))
+expr.map_elements(lambda value: (value,), return_dtype=pl.List(pl.Int64))
```

## What the rule reports

Inference produced List(Int64) on integer data and raised InvalidOperationError on two Null rows.

## How to fix it

Edit the code using the replacement in the example.
