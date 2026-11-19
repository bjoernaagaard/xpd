`np.percentile` takes percents from 0 to 100, so `0.95` asks for a value near the minimum. Use `np.quantile` for fractions, or pass 95.

```diff lang="python"
-upper = np.percentile(x, 0.95)
+upper = np.quantile(x, 0.95)
```

## What the rule reports

The rule reports `np.percentile(x, 0.95)` and `np.percentile(x, [0.25, 0.75])`. `np.percentile` takes percents from 0 to 100, so both calls ask for values near the minimum.

The rule leaves a list such as `[0.5, 99.5]` alone, where one number is a percent.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.quantile(x, 0.95)`.
