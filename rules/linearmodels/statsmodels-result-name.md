A linearmodels result does not have the statsmodels names, so `summary()`, `bse` and similar names raise. Use the linearmodels name that the message gives, such as `summary` without parentheses.

```diff lang="python"
 result = PanelOLS(y, X, entity_effects=True).fit()
-print(result.summary())
-errors = result.bse
+print(result.summary)
+errors = result.std_errors
```

## What the rule reports

The rule reports a statsmodels result name read on a linearmodels result, such as `summary()`
called, `bse`, `tvalues`, `resid`, `fittedvalues`, `aic`, `bic`, `llf`, `fvalue`, `f_pvalue`,
`ssr`, `mse_resid`, `cov_params`, `t_test`, `f_test`, `get_robustcov_results`, `summary2`, `save`
and 36 more.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `summary` for `summary()`, `std_errors` for `bse`, `tstats` for `tvalues`, `resids` for `resid` and `fitted_values` for `fittedvalues`. On the results of the panel models, OLS, IV2SLS, IVLIML, IVGMM, IVGMMCUE, IVJIVE and AbsorbingLS, also `resid_ss` for `ssr`, `s2` for `mse_resid`, `f_statistic.stat` for `fvalue` and `f_statistic.pval` for `f_pvalue`. The other names, and those four on the system and factor models, have no fix.

`summary` is a property, and on 7.1 and 7.0 `summary()` raised `TypeError: 'Summary' object is not callable` on the result of each of the 19 model classes. The other names are the 53 public names of a statsmodels 0.15.0 OLS result that none of the 19 results has, found with `dir()` and `hasattr`, and each raises `AttributeError`. The results of the factor models have no residuals under either name, so `resid` and `fittedvalues` are not reported on them.

| statsmodels | linearmodels |
| --- | --- |
| `summary()` | `summary` |
| `bse` | `std_errors` |
| `tvalues` | `tstats` |
| `resid` | `resids` |
| `fittedvalues` | `fitted_values` |
| `cov_params()` | `cov` |
| `fvalue`, `f_pvalue` | `f_statistic.stat`, `f_statistic.pval` |
| `ssr` | `resid_ss` |
| `mse_resid` | `s2` |
| `llf` | `loglik`, on the panel results |
| `t_test`, `f_test` | `wald_test`, on the panel and instrument results |
| `get_robustcov_results` | a new `fit(cov_type=...)` |

On `OLS` with the mroz data, each pair of the table from `bse` to `mse_resid` was equal to 1e-11 for an unadjusted fit with `debiased=True`, and `loglik` of `PooledOLS` was equal to `llf`. The rule reports 15 reads in 6 of the 950 public files. `.summary()` is called in 171 of the 950 files, and the rule reports it in the 3 where the result can be followed to a linearmodels model. Code under a `hasattr` check is a compatibility path and is left alone.
