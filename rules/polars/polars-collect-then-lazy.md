Collect executes this query before lazy starts a new plan. Keep the operations in one plan when they can run together.

```diff lang="python"
-lf.collect().lazy()
+lf
```

## What the rule reports

The collect call produced a DataFrame and lazy returned a new LazyFrame.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to drop `.collect().lazy()` when `collect` takes no arguments.
