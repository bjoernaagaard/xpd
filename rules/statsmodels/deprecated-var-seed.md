Statsmodels 0.15 renamed `seed` to `rng` on the VAR result methods
`simulate_var` and `irf_errband_mc`. The old keyword emits a
`FutureWarning` and forwards its value to the same argument.

```diff
-result.simulate_var(steps=10, seed=42)
+result.simulate_var(steps=10, rng=42)
```

## What the rule reports

The rule follows results from `VAR(...).fit(...)` and their local
assignments. It checks calls without argument unpacking or a separate
value for `rng`. Both methods returned identical arrays with the two
spellings for the same integer seed in 0.15.0.

## How to fix it

Use `xpd check --fix` to rename the keyword. This requires statsmodels 0.15
or later. If your application needs a particular random stream across future
statsmodels releases, pass an explicitly constructed random number generator
or `RandomState` as appropriate.

[Statsmodels RNG migration](https://www.statsmodels.org/stable/release/#spec-007-consistent-use-of-rng-for-randomness) ·
[simulate_var](https://www.statsmodels.org/stable/generated/statsmodels.tsa.vector_ar.var_model.VARResults.simulate_var.html) ·
[irf_errband_mc](https://www.statsmodels.org/stable/generated/statsmodels.tsa.vector_ar.var_model.VARResults.irf_errband_mc.html)
