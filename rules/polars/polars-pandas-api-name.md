Polars frames expose a different API from pandas. Use the matching Polars method or expression.

```diff lang="python"
-df.groupby("x")
+df.group_by("x")
```

## What the rule reports

Thirteen AttributeError results on Polars DataFrame, LazyFrame, and Series (`groupby`, `merge`, `apply`, `loc`, `iloc`, `iterrows`, `fillna`, `dropna`, `sort_values`, `reset_index`, `set_index`, `isna`, and `notna`).

## How to fix it

Edit the code using the replacement in the example.
