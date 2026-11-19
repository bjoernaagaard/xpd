Merging a group total back onto its frame builds a second frame and joins it, when `transform` already gives each row the total of its group. Write `df.groupby(key)[col].transform("sum")` instead.

```diff lang="python"
-result = df.merge(
-    df.groupby("region")["load"].sum().rename("total").reset_index(),
-    on="region",
-    how="left",
-    validate="many_to_one",
-)
+result = df.assign(total=lambda frame: frame.groupby("region")["load"].transform("sum"))
```

## What the rule reports

The rule reports `df.merge(df.groupby("k")["b"].sum()...)` and `df["k"].map(df.groupby("k")["b"].sum())`, where `transform` gives each row the aggregate of its group in one step. The rule reports an aggregation written inside the call on the same frame, and it misses an aggregate stored in a variable.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`transform` keeps the index and the order of the rows, where a merge can give the result a new index.
