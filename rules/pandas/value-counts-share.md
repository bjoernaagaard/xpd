Dividing `value_counts()` by the length computes shares by hand. Call `value_counts(normalize=True)` instead.

```diff lang="python"
-shares = df["region"].value_counts() / len(df["region"])
+shares = df["region"].value_counts(normalize=True)
```

## What the rule reports

The rule reports `df["k"].value_counts() / len(df["k"])`. The fix is unsafe, because it names the result `proportion` instead of `count`, and it leaves missing values out of the total.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["k"].value_counts(normalize=True)`.
