A loop over `groupby` that appends one summary for each group builds the result by hand. Use named aggregation, which returns one row per group in one call.

```diff lang="python"
-rows = []
-for region, part in df.groupby("region"):
-    rows.append((region, len(part), float(part["price"].median())))
-summary = pd.DataFrame(rows, columns=["region", "offers", "median_price"])
+summary = df.groupby("region", as_index=False).agg(
+    offers=("price", "size"),
+    median_price=("price", "median"),
+)
```

## What the rule reports

The rule reports a `for` loop over `df.groupby(...)` that appends a value which calls `mean`,
`median`, `sum`, `min`, `max`, `std`, `var`, `count`, `nunique`, `size` or `quantile`. It leaves
a loop alone that appends something else, such as a fitted model for each group.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The loop builds a frame for each group in Python, and every number is then pulled out and
converted by hand. Named aggregation runs each reduction on all groups at once and returns one
row per group in a table whose columns have names, so no record class and no `float()` is needed
to carry the result. For a share of rows that meet a condition, add the condition as a boolean
column first and take its `"mean"`.
