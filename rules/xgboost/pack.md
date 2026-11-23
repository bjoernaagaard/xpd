# XGBoost

The XGBoost pack checks the native Python API and the scikit-learn estimators
against the XGBoost 3.4.2 documentation.

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

## Autofixes

- `ineffective-max-quantile-batches` can remove a literal integer option.
- `removed-gpu-hist` can migrate an estimator whose only option is the old
  tree method. This fix is unsafe because it changes the device setup.
- Other configurations, reference changes, callback lifetimes and prediction
  choices require manual edits. Autofix verification checks that rewritten
  cases clear and that expected manual diagnostics remain.

## Runtime evidence

The examples ran with the XGBoost CPU wheel at version 3.4.1 on 20 rows with
two features. The GPU migration uses the documented parameter values.

| Rule | Observed result |
| --- | --- |
| [removed-fit-keywords](removed-fit-keywords.md) | Each old keyword raised TypeError at fit. The three keywords worked together on the constructor. |
| [removed-gpu-hist](removed-gpu-hist.md) | Estimator fit, native train and native cv rejected `gpu_hist`. |
| [ineffective-max-quantile-batches](ineffective-max-quantile-batches.md) | Passing 1 produced the same cuts as the default and warned that the setting had no effect. |
| [quantile-eval-without-train-ref](quantile-eval-without-train-ref.md) | The separate matrix raised ValueError. The evaluation list with the training matrix and a referenced validation matrix worked. |
| [reused-early-stopping-callback](reused-early-stopping-callback.md) | The first fit retained four rounds. Reusing the callback retained one. A fresh callback again retained four. |
| [native-predict-after-early-stopping](native-predict-after-early-stopping.md) | The best iteration was 0 and the model retained four rounds. Predictions for the first round differed from full model predictions. |
| [dmatrix-rebuilt-for-predict](dmatrix-rebuilt-for-predict.md) | Native and in-place predictions agreed for a NumPy array and the DMatrix built from it. |

[Python API reference](https://xgboost.readthedocs.io/en/stable/python/python_api.html)
