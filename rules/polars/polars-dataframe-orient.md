Nested list data can fit both DataFrame orientations. Set orient explicitly so the values land in the intended columns.

```diff lang="python"
-pl.DataFrame([[1, 2], [3, 4]], schema=["x", "y"])
+pl.DataFrame([[1, 2], [3, 4]], schema=["x", "y"], orient="row")
```

## What the rule reports

Inferred first row (1, 3). Explicit row orientation gave (1, 2).

## How to fix it

Edit the code using the replacement in the example.
