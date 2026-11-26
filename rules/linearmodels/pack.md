# linearmodels

The linearmodels pack catches formulas and arguments that fit the wrong model, mismatched covariance options, runtime errors on linearmodels 7.1, statsmodels attribute names used on linearmodels results, and removed APIs on files that import `linearmodels`. Rules fire only on calls and results bound to `linearmodels` imports.

## Formulas

- formula-without-constant
- effects-outside-panel-ols
- effects-keyword-with-formula
- formula-power-term
- first-difference-constant
- weights-column-name

## Panel models

- panel-ols-without-effects
- too-many-effects
- time-before-entity-index
- use-lsdv

## Covariance

- clustered-without-clusters
- cov-option-without-cov-type
- unknown-cov-type
- kernel-default-bandwidth
- panel-default-covariance

## Instruments and absorbed effects

- iv-missing-arguments
- iv-arguments-swapped
- iv-without-exog
- manual-two-stage
- absorb-not-categorical

## Results

- statsmodels-result-name
- invalid-compare-precision
- hand-written-hausman

## Removed and deprecated forms

- removed-in-linearmodels
- deprecated-in-linearmodels
