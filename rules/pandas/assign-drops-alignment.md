Assigning `.values`, `.to_numpy()` or `.tolist()` of another frame drops its index, so pandas pairs the rows by position. When the rows were in another order, every value landed on the wrong row. Assign the column itself so pandas pairs rows by label, or merge on a key column.

```diff lang="python"
-df["region"] = other["region"].values
+df["region"] = other["region"]
```

## What the rule reports

The rule reports `df["m"] = other["m"].values`, `.to_numpy()` and `.tolist()`, also through `df.loc[:, "m"]` and as a keyword of `df.assign`. The array has no index, so pandas pairs the rows by position. With the same 4 rows in another order, all 4 values landed on the wrong row. The rule leaves a column of the same frame alone, because it has the same index.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["b"] = other["b"]`, `df.loc[:, "b"] = other["b"]` and `assign(b=other["b"])`, which pair the rows by index label. A label that the two indexes do not share gives a missing value.

When the two indexes share no labels, merge the two frames on a key column instead.
