`np.cumsum(x)[-1]` builds every running total to read the last one, which was 8 times slower and adds rounding. Call `np.sum(x)`.

```diff lang="python"
-total = np.cumsum(amounts)[-1]
+total = np.sum(amounts)
```

## What the rule reports

The rule reports `np.cumsum(x)[-1]` and `np.cumprod(x)[-1]`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.sum(x)`, `np.prod(x)`.
