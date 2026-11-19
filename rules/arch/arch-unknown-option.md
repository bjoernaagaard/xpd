This text is not one of the values the option takes, so arch raises ValueError. Use one of the values that the message lists.

```diff lang="python"
-gjr = arch_model(returns, vol="gjr", dist="student")
-res = gjr.fit(cov_type="mle")
-adf = ADF(prices, trend="nc")
+gjr = arch_model(returns, vol="garch", o=1, dist="t")
+res = gjr.fit(cov_type="classic")
+adf = ADF(prices, trend="n")
```

## What the rule reports

The rule reports a string that is not one of the option's values, for each option in the table after the
example. arch raises ValueError on such a string.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The fix is unsafe even though the old call raises. It writes `trend="n"` for `trend="nc"` in the classes that take `"n"`. Every other option has no fix, because the call does not show which accepted option was meant.

The lists in the rule are the ones that arch 9.0.0 uses, and none is typed by hand. arch checks
each option against a table of accepted spellings, without regard to case. A script read the
tables in three steps. First, it wrapped arch's check, called each API once and kept the table
that the check was handed. Second, it ran each API with every spelling of its table, in lower
and in upper case. Third, it ran each API with 28 other strings, and each of the 28 raised
ValueError.

The tables hold more than the signatures show. For example, `vol="constant"` builds a model with
a constant variance, and `conf_int` also takes `"var"`, `"cov"`, `"debiased"` and
`"bias-corrected"`. Moreover, `method` of the unit root tests also takes `"t"` and `"tstat"`, and
`bootstrap="moving_block"` is the same as `"moving block"`.

| Option | Values on arch 9.0.0 |
| --- | --- |
| `arch_model(mean=)` | `constant`, `zero`, `ls`, `ar`, `arx`, `har`, `harx` |
| `arch_model(vol=)` | `garch`, `arch`, `egarch`, `figarch`, `fiaparch`, `aparch`, `harch`, `ngarch`, `constant` |
| `arch_model(dist=)` | `normal`, `gaussian`, `t`, `studentst`, `skewstudent`, `skewt`, `ged`, `generalized error` |
| `trend=` of `ADF`, `engle_granger`, `phillips_ouliaris`, `DynamicOLS`, `FullyModifiedOLS`, `CanonicalCointegratingReg` | `n`, `c`, `ct`, `ctt` |
| `trend=` of `DFGLS`, `KPSS` | `c`, `ct` |
| `trend=` of `PhillipsPerron` | `n`, `c`, `ct` |
| `trend=` of `VarianceRatio` | `n`, `c` |
| `trend=` of `ZivotAndrews` | `c`, `t`, `ct` |
| `method=` of `ADF`, `DFGLS`, `ZivotAndrews`, `engle_granger` | `aic`, `bic`, `t-stat`, `maic`, `t`, `tstat` |
| `method=` of `DynamicOLS` | `aic`, `bic`, `hqic` |
| `PhillipsPerron(test_type=)` | `tau`, `rho` |
| `phillips_ouliaris(test_type=)` | `za`, `zt`, `pu`, `pz` |
| `auto_bandwidth(kernel=)` | `ba`, `bartlett`, `nw`, `pa`, `parzen`, `gallant`, `qs`, `andrews` |
| `MCS(method=)` | `r`, `max` |
| `bootstrap=` of `SPA`, `StepM`, `MCS`, `RealityCheck` | `stationary`, `sb`, `circular`, `cbb`, `moving block`, `mbb` |
| `fit(cov_type=)` | `robust`, `classic` |
| `fit(disp=)` | `off`, `final`, or a bool |
| `forecast(method=)`, `hedgehog_plot(method=)` | `analytic`, `simulation`, `bootstrap` |
| `forecast(align=)` | `origin`, `target`, `horizon` |
| `hedgehog_plot(plot_type=)` | `volatility`, `mean` |
| `conf_int(method=)` | `basic`, `percentile`, `studentized`, `norm`, `var`, `cov`, `bc`, `debiased`, `bias-corrected`, `bca` |
| `conf_int(tail=)` | `two`, `lower`, `upper` |

The documents have no `since`, so xpd also reports on a project with an older arch. Before 9.0
some of the options took any string and fell back to one value without a warning.

| Call | Releases | Result | Value with a right option |
| --- | --- | --- | --- |
| `fit(cov_type="mle")` on 1,500 daily S&P 500 returns | 4.19, 5.0, 6.1.0, 6.2.0, 8.0.0 | the standard errors of `"classic"`, 0.00843 for omega | `"robust"`: 0.01329 |
| `PhillipsPerron(y, test_type="z")` | 4.19, 5.0, 6.1.0, 6.2.0, 8.0.0 | the statistic of `"tau"`, -1.136 | `"rho"`: -2.0998 |
| `MCS(losses, 0.1, method="range")` | 5.0, 6.1.0, 6.2.0, 8.0.0 | the p-values of `"max"` | `"R"`: other p-values |

Releases before 9.0 took some options in one case only. For example, `ADF(y, method="AIC")`
raised on 8.0.0 and runs on 9.0.0. The rule follows 9.0.0 and does not report it, and it does not
report a value that only 9.0 has, such as `method="maic"` or `align="horizon"`, either.

The public code had 13 reports, and each was a value that arch refuses:

- `trend="nc"`, which is the old statsmodels name
- `vol="AVGARCH"` and `disp="on"`
- `method="stationary"` on `MCS`
- `ZivotAndrews(y, method="c")`, where the trend was written as the method
