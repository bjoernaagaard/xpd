Chained with_columns calls may split work into separate steps. Combine independent expressions and keep later calls for expressions that use newly created columns.

```diff lang="python"
-df.with_columns(a=expr + 1).with_columns(b=expr + 2)
+df.with_columns(a=expr + 1, b=expr + 2)
```

## What the rule reports

Independent expressions gave equal frames.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to combine chained `with_columns` calls when the second call does not depend on columns created in the first.
