# statsmodels

The statsmodels pack checks regression and time-series code against statsmodels 0.15.0.
The time-series migration rules apply from 0.15. The other rules check behavior
documented in 0.15.0.

## Regression

- ols-ignored-weights
- statsmodels-formula-power-term
- glm-logged-exposure
- compare-f-test-robust-covariance
- rolling-fit-params-only

## Time-series migrations

- removed-tsa-arguments
- deprecated-var-seed

## Rule scope

The rules use import and local assignment evidence. They cover standard module aliases
and direct imports under the API names. Computed formulas, dynamic keyword names, and
model results hidden behind arbitrary factories can require manual review.

[Statsmodels API reference](https://www.statsmodels.org/stable/api.html) ·
[0.15.0 release notes](https://www.statsmodels.org/stable/release/)
