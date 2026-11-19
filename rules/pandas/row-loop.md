`iterrows()` builds a Series for every row, and a loop that only computes from row values runs Python once for every row. Compute on whole columns instead.

```diff lang="python"
-total = sum(row.load * row.weight for row in df.itertuples())
+total = (df["load"] * df["weight"]).sum()
```

## What the rule reports

The rule reports a loop over `iterrows()`, a loop over `itertuples()` or `to_dict("records")` that only computes from the row values, and `for i in range(len(df))` with `df.iloc[i]`. It does not run on test files.

The rule leaves alone a loop over `itertuples()` or `to_dict("records")` that hands the row or its values to a function, as in `orders = [Order(id=row["id"], total=row["total"]) for row in df.to_dict("records")]`, because the loop turns rows into Python objects. A cast such as `int(row["a"])` and a list method such as `out.append(...)` don't count as a function here. The rule reports `iterrows()` in every loop, because `iterrows()` builds a Series for each row. Use `itertuples()` or `to_dict("records")` when each row has to become an object.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The replacement sums numeric columns without missing values. When you replace a Python reduction, check the missing values and the floating point rounding. Keep a loop when its work can't be written on columns.
