Polars 2.1.0 emits UserWarning for comparison to None. Use is_null to test for nulls.

```diff lang="python"
-expr == None
+expr.is_null()
```

## What the rule reports

| Form | Replacement example | Proof on 2.1.0 |
| --- | --- | --- |
| `expr == None` | `expr.is_null()` | UserWarning under -W error |
| `expr != None` | `expr.is_not_null()` | UserWarning under -W error |

## How to fix it

Use `xpd check --fix --unsafe-fixes` to apply the offered rewrite after checking the intended result.
