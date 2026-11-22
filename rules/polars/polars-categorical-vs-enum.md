Categorical accepts every string label. Use Enum when you have a fixed list and want unknown labels to raise.

```diff lang="python"
-pl.Series(["open", "closed"], dtype=pl.Categorical)
+pl.Series(["open", "closed"], dtype=pl.Enum(["open", "closed"]))
```

## What the rule reports

Categorical accepted typo. Enum raised InvalidOperationError for 1 unknown value.

## How to fix it

Edit the code using the replacement in the example.
