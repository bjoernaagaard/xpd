A distribution's `pdf`, `cdf` or `sf` underflows to 0 far in a tail, so `np.log` of it gives -inf there. Call the log method, such as `logpdf`, which keeps the digits.

```diff lang="python"
-ll = np.log(stats.norm.pdf(resid, scale=sigma)).sum()
+ll = stats.norm.logpdf(resid, scale=sigma).sum()
```

## What the rule reports

The rule reports `np.log` of the `pdf`, `pmf`, `cdf` or `sf` of a `scipy.stats` distribution,
written as `stats.norm`, imported by name as `norm`, or frozen in the same expression as
`stats.norm(0, 2)`. Each method has its own list of distributions, and each distribution on a
list was measured in a tail. `np.log(stats.binom.pmf(k, n, p))` is not reported, because `binom.logpmf` lost up to 3.6
digits more near the mode.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `dist.logpdf(...)`, `dist.logpmf(...)`, `dist.logcdf(...)`, `dist.logsf(...)`.
