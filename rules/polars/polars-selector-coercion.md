A selector combined with col by &, | or ^ becomes a value expression in Polars 2.1.0. Use selectors.by_name to combine column selections.

```diff lang="python"
-cs.numeric() | pl.col("x")
+cs.numeric() | cs.by_name("x")
```

## What the rule reports

Taking the union of numeric columns and a string column as a value expression raised SchemaError. Two selectors selected 3 columns.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to rewrite `pl.col(...)` with string column names to `by_name(...)` on the selector module.
