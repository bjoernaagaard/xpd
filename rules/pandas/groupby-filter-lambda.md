`groupby(...).filter(lambda g: len(g) > n)` runs Python once for every group only to count its rows. Compare `transform("size")` with the limit and select with the mask.

```diff lang="python"
-selected = df.groupby("region").filter(lambda group: len(group) > 10)
+selected = df[df.groupby("region")["region"].transform("size") > 10]
```

## What the rule reports

The rule reports `df.groupby("k").filter(lambda g: len(g) > 10)`, which runs Python once per group to count its rows.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df[df.groupby("k")["k"].transform("size") > 10]`.

The replacement assumes that no group key is missing. Check how each form treats a missing key before you rewrite a filter.
