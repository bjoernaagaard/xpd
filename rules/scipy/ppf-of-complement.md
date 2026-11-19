`ppf(1 - p)` rounds away the digits of a small `p`, so it loses precision far in the tail. Call `isf(p)`.

```diff lang="python"
-crit = stats.norm.ppf(1 - alpha / 2)
+crit = stats.norm.isf(alpha / 2)
```

## What the rule reports

The rule reports `dist.ppf(1 - p, ...)` and `dist.isf(1 - p, ...)` for a continuous
`scipy.stats` distribution from a measured list, and `special.ndtri(1 - p)`. The old form lost 3
or more digits for every p below 5e-7. A probability written as a number, such as `1 - 0.05`,
is left alone unless it is below 5e-7.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `dist.isf(p, ...)`, `dist.ppf(p, ...)`, `-special.ndtri(p)`.
