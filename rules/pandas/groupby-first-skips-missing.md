`groupby(...).first()` and `last()` take the first value that is not missing in each column, so one result row can mix values from different rows. Pass `skipna=False` when you want the whole first or last row.

```diff lang="python"
-first = df.groupby("region").first()
+first = df.groupby("region").first(skipna=False)
```

## What the rule reports

The rule reports `df.groupby("k").first()` and `last()`, which skip missing values per column, so one result row can mix values from different rows. The rule leaves `df.groupby("k")["b"].last()` alone, because one column can't mix rows.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `first(skipna=False)` and `last(skipna=False)`, which change the result on purpose to the whole first or last row.

`skipna=False` keeps the missing values of the first row.
