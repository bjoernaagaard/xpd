A statsmodels rolling regression can compute coefficients without the
statistics needed for inference. The default inverse solver uses the same
coefficient calculation when `params_only=True`.

```diff
-params = RollingOLS(y, X, window=60).fit().params
+params = RollingOLS(y, X, window=60).fit(params_only=True).params
```

## What the rule reports

The rule checks a zero-argument `fit()` on a proven `RollingOLS` or `RollingWLS`
model when the caller immediately reads `params`. The two fit modes produced
identical coefficient arrays for both models in 0.15.0, including the missing
values before the first full window.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to set `params_only=True`. Skipping
inference can change whether the fit raises. With two observations and two
columns per window in statsmodels 0.15.0, the full fit raised
`FloatingPointError` under `numpy.errstate(all="raise")` and the
parameter-only fit returned coefficients. Keep the full fit when you need
inference statistics or depend on its failure behavior.

[Rolling regression fit options](https://www.statsmodels.org/stable/examples/notebooks/generated/rolling_ls.html#fit-options) ·
[Rolling fit implementation](https://www.statsmodels.org/stable/_modules/statsmodels/regression/rolling.html)
