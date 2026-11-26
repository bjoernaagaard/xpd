# scikit-lego

The scikit-lego pack catches silent argument mistakes, pipeline ordering traps, broken scorer calls, and removed or superseded scikit-lego 0.9 APIs on files that import `sklego`. Rules fire only on names bound to `sklego` imports.

## Arguments that give a wrong number

- thresholder-score
- estimator-transformer-proba
- column-capper-fraction
- dict-mapper-not-int
- regression-outlier-column-name
- pca-outlier-after-scaler
- outlier-threshold
- grouped-shrinkage-arguments

## Rows and fits

- add-lags-positive
- decay-on-shuffled-rows
- outlier-remover-before-model
- decay-without-sample-weight
- zero-inflated-roles

## Scorers, datasets and helpers

- scorer-called-as-metric
- dataset-frame-and-xy
- log-step-extra-bare
- selector-without-feature-names
- debug-pipeline-default-log

## Removed, deprecated and superseded forms

- removed-in-sklego
- deprecated-in-sklego
- superseded-estimator

## Speed and repeatable runs

- group-time-series-split-folds
- unseeded-random-estimator
