A lambda in `transform` that calls `shift`, `diff` or a similar method runs Python once for every group. Call the method on the groupby directly.

```diff lang="python"
-previous = df.groupby("region")["load"].transform(lambda group: group.shift(1))
+previous = df.groupby("region")["load"].shift(1)
```

## What the rule reports

The rule reports `g["b"].transform(lambda v: v.shift(1))`, which runs Python once per group where the groupby method runs on all groups at once.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `g["b"].shift(1)`, `g["b"].cumsum()`. The method can give another dtype or index type on an empty or all-missing group.

The fixes for `cumsum` and `cumprod` are unsafe, because the groupby methods add up a running total with compensated summation, which keeps track of the rounding error of each addition. A total over values far apart in size can therefore change. The rule leaves `shift(freq=)` alone, because it moves the index.
