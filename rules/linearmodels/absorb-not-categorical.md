In linearmodels, `AbsorbingLS` absorbs a column as fixed effects only when its dtype is categorical, so a numeric id becomes one continuous regressor. Cast the column to `category` before you pass it.

```diff lang="python"
-model = AbsorbingLS(data.y, data[["x"]], absorb=data[["firm"]])
+model = AbsorbingLS(data.y, data[["x"]], absorb=data[["firm"]].astype("category"))
```

## What the rule reports

The rule reports `AbsorbingLS(..., absorb=...)` in a file that never says `category`,
`Categorical` or `cut(`. `absorb` reads the dtype of each column. A categorical column is absorbed
as one effect for each category, and any other column as one continuous regressor.

## How to fix it

xpd has no automatic fix, so make the change by hand.

With 200 firms, a firm effect in the regressor and a true coefficient of 1.0, no form raised an
error or a warning:

| `absorb` | Coefficient |
| --- | --- |
| the integer firm id | 3.397 |
| none | 3.398 |
| the firm id as a category | 1.022 |

A text column raises `TypeError` on 7.1, and 7.0 took it like the integers. One of the 4 public files
that the rule reports turns its columns into text with `astype(str)` right before the call. The
code doesn't show a dtype, so the rule is info. Data that is categorical when it is loaded is
reported without need.
