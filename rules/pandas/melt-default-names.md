`melt` without `var_name` names the column of old column names `variable`, which says nothing about them. Name it with `var_name`, and name the values with `value_name`.

```diff lang="python"
-long = df.melt(id_vars="id")
+long = df.melt(id_vars="id", var_name="year", value_name="sales")
```

## What the rule reports

The rule reports `melt` without `var_name`, which names the column of old column names `variable`. The column of their values is named `value` unless the call passes `value_name`.

## How to fix it

xpd has no automatic fix, so make the change by hand.
