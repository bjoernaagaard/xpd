Statsmodels `compare_f_test` compares residual sums of squares. It warns when
either fit uses robust covariance and still computes the classical F-test
p-value under the assumptions of homoskedasticity and no autocorrelation.

```diff
-full.compare_f_test(restricted)
+full.wald_test("x = 0", scalar=True)
```

## What the rule reports

The rule checks two statsmodels OLS results when either uses a literal
robust `cov_type`, including `"cluster-jk"` and `"cluster-crv3"`. It reads
the covariance passed to `fit` by position or keyword and follows
`get_robustcov_results` chained directly after the fit. Both inline
results and local assignments are supported. In 0.15.0,
setting HC3 on either fit emitted `InvalidTestWarning` and returned the
same p-value as the classical comparison.

## How to fix it

Fit the full model with the desired robust covariance and express the nested
model as coefficient restrictions. For example, if the full model adds only
`x` to the restricted model, test `x = 0`. Check that the two specifications
use the same observations.

[compare_f_test](https://www.statsmodels.org/stable/generated/statsmodels.regression.linear_model.RegressionResults.compare_f_test.html) ·
[wald_test](https://www.statsmodels.org/stable/generated/statsmodels.regression.linear_model.RegressionResults.wald_test.html)
