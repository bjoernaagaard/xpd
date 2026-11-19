`concat(axis=1)` pairs rows by index label and keeps every label of every frame, so one differing label adds a row of missing values and turns int columns into floats. Pass `join="inner"` to keep only shared labels, or `join="outer"` to say the missing values are meant.

```diff lang="python"
-both = pd.concat([left, right], axis=1)
+both = pd.concat([left, right], axis=1, join="inner")
```

## What the rule reports

The rule reports `pd.concat([...], axis=1)` without `join=`, which pairs rows by index label and keeps every label of every frame. The table shows two frames of 4 rows whose labels differ in one row.

| Call | Rows | Dtype of both columns |
| --- | --- | --- |
| `pd.concat([left, right], axis=1)` | 5, with a missing value | `float64` |
| `pd.concat([left, right], axis=1, join="inner")` | 3 shared rows | `int64` |

## How to fix it

xpd has no automatic fix, so make the change by hand.

Write `join="outer"` when you want the missing values.
