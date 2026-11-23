LightGBM's early stopping callback disables itself in DART mode. This also
applies when `train()` or `cv()` creates the callback from a positive
`early_stopping_round` parameter or one of its aliases.

```diff
-lgb.train({"boosting": "dart", "early_stopping_round": 10}, train_set, valid_sets=[valid_set])
+lgb.train({"boosting": "gbdt", "early_stopping_round": 10}, train_set, valid_sets=[valid_set])
```

## How to fix it

Choose GBDT and supply held-out validation data and a metric if you need
early stopping. A callback with no evaluation results raises `ValueError`
before it checks for DART. To keep DART, choose the iteration count using
separate validation and omit the early stopping callback.

The rule checks native parameters and explicit LightGBM callbacks on
estimators built with a literal DART setting. The callback checks all
boosting aliases for `"dart"`, even when another alias says `"gbdt"`.
Make the change by hand because selecting a boosting strategy changes the
model.

[Early stopping API](https://lightgbm.readthedocs.io/en/v4.7.0/pythonapi/lightgbm.early_stopping.html) |
[Training engine](https://lightgbm.readthedocs.io/en/v4.7.0/_modules/lightgbm/engine.html)
