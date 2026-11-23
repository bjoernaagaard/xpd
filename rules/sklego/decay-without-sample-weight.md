`DecayEstimator` passes `sample_weight` to the fit of the model it wraps, so wrapping a Pipeline or a model without `sample_weight` raises ValueError. Put the `DecayEstimator` around the last step, inside the pipeline.

```diff lang="python"
-model = DecayEstimator(make_pipeline(StandardScaler(), Ridge()))
+model = make_pipeline(StandardScaler(), DecayEstimator(Ridge()))
```

## What the rule reports

The rule reports `DecayEstimator` around a `Pipeline`, and around a scikit-learn model whose `fit`
takes no `sample_weight`, such as `KNeighborsRegressor`, `GaussianProcessRegressor` and
`LinearDiscriminantAnalysis`. `DecayEstimator` passes its weights as
`fit(X, y, sample_weight=...)`.

## How to fix it

`xpd check --fix --unsafe-fixes` moves `DecayEstimator` around the last step of a two-step `make_pipeline` or `make_debug_pipeline`. Edit other pipeline layouts and unweighted models by hand.

The pipeline raised ValueError "Pipeline.fit does not accept the sample_weight parameter", and
`KNeighborsRegressor` raised TypeError "Model KNeighborsRegressor.fit() does not have
'sample_weight'". With the `DecayEstimator` around the last step, the pipeline fitted.

The list of models has 22 names, among them `PassiveAggressiveClassifier`,
`PassiveAggressiveRegressor`, `CCA` and `PLSCanonical`. It holds every classifier and regressor of
`sklearn.utils.all_estimators()` on scikit-learn 1.9.1 that takes no argument to build and whose
`fit` raised that TypeError inside a `DecayEstimator`.
