Polars raises ComputeError for an out-of-bounds list offset by default. Pass null_on_oob=True when shorter lists are expected.

```diff lang="python"
-pl.Series([[1], [2, 3]]).list.get(1)
+pl.Series([[1], [2, 3]]).list.get(1, null_on_oob=True)
```

## What the rule reports

Index 1 raised ComputeError for a one-item list. null_on_oob=True returned [None, 3].

## How to fix it

Edit the code using the replacement in the example.
