Polars 2.0 removed several casts that need an explicit conversion. Parse temporal strings with the string namespace and construct nested values explicitly.

```diff lang="python"
-pl.Series(["2025-01-01"]).cast(pl.Date)
+pl.Series(["2025-01-01"]).str.to_date()
```

## What the rule reports

| Form | Replacement example | Proof on 2.1.0 |
| --- | --- | --- |
| `pl.Series(["2025-01-01"]).cast(pl.Date)` | `pl.Series(["2025-01-01"]).str.to_date()` | InvalidOperationError |
| `pl.Series([1]).cast(pl.Categorical)` | `pl.Series([1]).cast(pl.String).cast(pl.Categorical)` | ComputeError |
| `pl.Series([1]).cast(pl.List(pl.Int64))` | `pl.Series([[1]], dtype=pl.List(pl.Int64))` | InvalidOperationError |
| `pl.Series(["a"]).cast(pl.Categorical).cast(pl.Int64)` | `pl.Series(["a"]).cast(pl.Categorical).to_physical()` | ComputeError |

## How to fix it

Edit the code using the replacement in the example.
