Polars 2.1.0 emits PolarsInefficientMapWarning when map_elements wraps addition in this lambda. Add the value directly with a Polars expression.

```diff lang="python"
-expr.map_elements(lambda value: value + 1, return_dtype=pl.Int64)
+expr + 1
```

## What the rule reports

PolarsInefficientMapWarning under -W error.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to replace the `map_elements` addition lambda with direct expression addition.
