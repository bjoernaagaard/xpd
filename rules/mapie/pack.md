# MAPIE

The MAPIE pack catches removed MAPIE 0.x calls, broken conformal coverage guarantees, invalid arguments, duplicate model fits, and unseeded resampling on files that import [MAPIE](https://mapie.readthedocs.io/). Rules fire only on calls bound to `mapie` imports.

## The 0.x API

- removed-in-mapie
- deprecated-in-mapie

## Coverage that does not hold

- alpha-as-confidence-level
- calibrate-on-training-rows
- coverage-on-seen-rows
- shuffled-time-series-split

## Arguments that raise

- prefit-estimator-not-fitted
- quantile-estimator-loss
- predict-function-returns-labels
- risk-control-method

## Work done twice

- predict-and-predict-interval
- refit-per-confidence-level

## Reproducible results

- unseeded-resampling
