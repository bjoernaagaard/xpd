Polars 2.1.0 emits PerformanceWarning when code reads LazyFrame.schema. Use collect_schema to make schema resolution explicit.

```diff lang="python"
-lf.schema
+lf.collect_schema()
```

## What the rule reports

| Form | Replacement example | Proof on 2.1.0 |
| --- | --- | --- |
| `lf.schema` | `lf.collect_schema()` | PerformanceWarning. The explicit replacement returned the same value and type. |
| `lf.columns` | `lf.collect_schema().names()` | PerformanceWarning. The explicit replacement returned the same value and type. |
| `lf.dtypes` | `lf.collect_schema().dtypes()` | PerformanceWarning. The explicit replacement returned the same value and type. |
| `lf.width` | `len(lf.collect_schema())` | PerformanceWarning. The explicit replacement returned the same value and type. |

## How to fix it

Use `xpd check --fix` to apply the safe rewrite when the report offers one.
