A loop that computes each value from the one before runs Python once for every step. Use `scipy.signal.lfilter` for a linear step and `np.cumsum` for a running total.

```diff lang="python"
-log_price = np.empty(days + 1)
-log_price[0] = first
-for t in range(1, days + 1):
-    log_price[t] = intercept + slope * log_price[t - 1] + sd * rng.standard_normal()
+shocks = intercept + sd * rng.standard_normal(days)
+later, _ = scipy.signal.lfilter([1.0], [1.0, -slope], shocks, zi=[slope * first])
+log_price = np.concatenate([[first], later])
```

## What the rule reports

The rule reports a `for` loop over `range` that writes `x[t]` from `x[t - 1]` of the same array.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The loop is a linear recurrence, which `scipy.signal.lfilter` computes in one call. On NumPy
2.5.5 and SciPy 1.18.1 the two forms differed by 4e-15 over 5,000 steps, with the same random
draws. A running sum is `np.cumsum`, a running product `np.cumprod`, and a running maximum
`np.maximum.accumulate`. A step that is not linear in the earlier value has to stay a loop.
