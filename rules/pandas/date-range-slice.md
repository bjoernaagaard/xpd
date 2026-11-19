Slicing off the last step of `date_range` with `[:-1]` does what `inclusive="left"` does. Pass `inclusive="left"` instead.

```diff lang="python"
-steps = pd.date_range("2024-03-30", "2024-04-01", freq="15min")[:-1]
+steps = pd.date_range("2024-03-30", "2024-04-01", freq="15min", inclusive="left")
```

## What the rule reports

The rule reports `pd.date_range(start, end, freq="15min")[:-1]`, which drops the end with a slice where `inclusive="left"` leaves it out. The rule leaves a range with `periods=` alone.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.date_range(start, end, freq="15min", inclusive="left")`.

The fix is unsafe when the end isn't a step of the range. Then the slice drops the last real step, and `inclusive="left"` keeps it. The fix adds an argument after the last one, so the rule leaves a call alone that ends in a trailing comma or holds a comment.
