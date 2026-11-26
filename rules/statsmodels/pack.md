# statsmodels

The statsmodels pack checks regression, formula, and time-series migration code against [statsmodels 0.15.0](https://www.statsmodels.org/stable/api.html) on files that import `statsmodels`. Rules rely on import and local assignment bindings.

## Regression

- ols-ignored-weights
- statsmodels-formula-power-term
- glm-logged-exposure
- compare-f-test-robust-covariance
- rolling-fit-params-only

## Time-series migrations

- removed-tsa-arguments
- deprecated-var-seed
