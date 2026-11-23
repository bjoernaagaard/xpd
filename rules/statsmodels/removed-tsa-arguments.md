Statsmodels 0.15 removed `old_names` from `AutoReg` and `ar_select_order`
and `verbose` from `grangercausalitytests`. These keywords now raise
`TypeError`. A literal `nlags=None` in `kpss` now raises `ValueError`.

```diff
-AutoReg(y, lags=2, old_names=True)
-grangercausalitytests(series, maxlag=2, verbose=False)
-kpss(y, nlags=None)
+AutoReg(y, lags=2)
+grangercausalitytests(series, maxlag=2)
+kpss(y, nlags="auto")
```

## How to fix it

Remove `old_names` and update any code that reads legacy parameter labels.
Remove `verbose` and use the returned Granger test dictionary if you need
to display results. Replace `nlags=None` with `nlags="auto"` to keep the
lag selection that statsmodels 0.14 used for `None`.

Use `xpd check --fix --unsafe-fixes` for the KPSS edit. It changes a call
that raises into one that runs.

[Statsmodels 0.15 removals](https://www.statsmodels.org/stable/release/#deprecated-parameters-removed-entirely)
