# XGBoost

The XGBoost pack checks native Booster and scikit-learn estimator code against [XGBoost 3.4](https://xgboost.readthedocs.io/en/stable/python/python_api.html) on files that import `xgboost`. Safe and unsafe autofixes cover `ineffective-max-quantile-batches` and simple `removed-gpu-hist` rewrites.

## Version migrations

- removed-fit-keywords
- removed-gpu-hist
- ineffective-max-quantile-batches

## Training and evaluation

- quantile-eval-without-train-ref
- reused-early-stopping-callback

## Prediction

- native-predict-after-early-stopping
- dmatrix-rebuilt-for-predict
