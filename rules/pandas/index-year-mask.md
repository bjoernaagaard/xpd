Selecting a year with `df.index.year == 2023` computes the year of every row. On a DatetimeIndex, select it with a label, as in `df.loc["2023"]`.

```diff lang="python"
-year = df[df.index.year == 2023]
+year = df.loc["2023"]
```

## What the rule reports

The rule reports `df[df.index.year == 2023]`, which computes the year of every row to build a mask.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.loc["2023"]`. On a PeriodIndex of years the label is one period, so the fix returns the value where the mask returned a Series of one row.

`df.loc["2023"]` selects by label and needs a sorted DatetimeIndex.
