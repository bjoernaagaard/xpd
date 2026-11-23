LightGBM's prediction engine normally checks that the number of features
matches training. With `predict_disable_shape_check=True`, mismatched
input can yield incorrect predictions. Some estimator inputs also go
through a separate scikit-learn check before they reach the engine.

```diff
-booster.predict(features, predict_disable_shape_check=True)
+booster.predict(features, validate_features=True)
```

## How to fix it

Supply the features in training order and keep the count check enabled.
For a pandas DataFrame, `validate_features=True` also checks the feature
names against training. The example assumes a pandas DataFrame.

The unsafe fix sets the flag to `False`. This restores the check for a
Booster, an estimator, or a CVBooster. Calls that relied on mismatched
inputs may then raise until you fix their feature schema.

[Prediction parameters](https://lightgbm.readthedocs.io/en/v4.7.0/Parameters.html#predict_disable_shape_check) |
[Booster prediction API](https://lightgbm.readthedocs.io/en/v4.7.0/pythonapi/lightgbm.Booster.html#lightgbm.Booster.predict)
