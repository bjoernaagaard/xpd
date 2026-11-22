Using linearmodels to fit a second regression on the fitted values of a first one is two stage least squares by hand, and its standard errors come from the wrong residuals. Use `IV2SLS`.

```diff lang="python"
-first = OLS(data.educ, data[["const", "exper", "fatheduc", "motheduc"]]).fit()
-second = OLS(data.lwage, pd.concat([data[["const", "exper"]], first.fitted_values], axis=1)).fit()
+result = IV2SLS(data.lwage, data[["const", "exper"]], data.educ, data[["fatheduc", "motheduc"]]).fit()
```

## What the rule reports

The rule reports a model whose `exog` holds `.fitted_values` of another fit. The model is `OLS` or
a panel class, or `IV2SLS`, `IVLIML` or `IVGMM` with `None` for `endog` and for `instruments`,
which is least squares. Such a model is two stage least squares by hand. The coefficient is right,
but the standard errors come from residuals that use the fitted values in place of the regressor.

## How to fix it

xpd has no automatic fix, so make the change by hand.

| Measurement | By hand | `IV2SLS` |
| --- | --- | --- |
| coefficient of `educ` on the mroz data | 0.0664 | 0.0664 |
| unadjusted standard error on the mroz data | 0.0327 | 0.0311 |
| coverage of the 95% interval, over 2,000 draws of 500 rows with one instrument | 99.6% | 95.7% |

In the draws, the standard error by hand was 1.37 times the other on average. Which way the error
goes depends on the data. The rule sees only the form where the fitted values are written in the
call, and it misses a second stage that reads them from a column stored earlier. Neither set of
code has the form the rule reads.
