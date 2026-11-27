# scikit-learn

The scikit-learn pack catches train and test leakage, split and group errors, swapped metric arguments, removed scikit-learn 1.9 APIs, duplicate fits, and missing random seeds on files that import `sklearn`. Rules fire only on names bound to `sklearn` imports.

## Leakage

- scaler-fit-before-split
- prefit-before-cross-validation
- fit-on-test
- fit-before-split
- cross-val-on-fitted-data
- tune-on-test
- stateful-function-transformer

## Splits and groups

- groups-ignored
- missing-groups
- leave-one-out-score

## Metrics and scoring

- auc-from-labels
- metric-arguments-swapped
- score-on-training-rows
- unknown-scoring-name

## Removed and deprecated forms

- removed-in-sklearn
- deprecated-in-sklearn
- removed-metric-api
- deprecated-metric-api
- fit-params-keyword
- column-transformer-removed

## Pipeline and metadata routing

- scaled-target-without-inverse
- metadata-routing
- pipeline-fit-params
- pipeline-step-form
- transformed-target-pair
- pandas-output-sparse

## Work done twice

- predict-per-row
- score-after-predict
- predict-and-predict-proba
- cross-val-score-per-metric
- vectorizer-fit-then-transform
- refit-per-n-estimators
- tsne-exact

## Reproducible results

- random-state-without-shuffle
- missing-random-state
