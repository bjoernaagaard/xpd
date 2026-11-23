Polars 2.1.0 raises TypeError when this Python builtin tries to size or iterate an Expr. Use the corresponding expression method.

```diff lang="python"
-len(expr)
+expr.len()
```

## What the rule reports

Five TypeError results on Expr. All five builtins worked on Series.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to replace the Python builtin with the matching Polars expression method.
