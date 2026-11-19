Subtracting a column's own `shift()` computes the difference from the previous row by hand. Call `diff()` instead.

```diff lang="python"
-change = df["load"] - df["load"].shift()
+change = df["load"].diff()
```

## What the rule reports

The rule reports `df["b"] - df["b"].shift()`. The fix is unsafe, because `diff` wraps around on a column of unsigned integers and returns booleans for a boolean column.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["b"].diff()`.
