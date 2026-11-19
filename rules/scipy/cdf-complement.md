`1 - dist.cdf(x)` drops the digits of a small right tail and gives 0 once the cdf rounds to 1. Call `dist.sf(x)`, which keeps them.

```diff lang="python"
-pvalue = 2 * (1 - stats.norm.cdf(np.abs(z)))
+pvalue = 2 * stats.norm.sf(np.abs(z))
```

## What the rule reports

The rule reports `1 - dist.cdf(...)` and `1 - dist.sf(...)` for a `scipy.stats` distribution
from a measured list. For `stats.norm`, `1 - cdf` lost 3 or more digits above 4.25 and gave 0
above 8.3. `lognorm`, `f`, `poisson`, `binom`, `nbinom` and `geom` are not on the list, because
their `sf` lost digits too.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `dist.sf(...)`, `dist.cdf(...)`.
