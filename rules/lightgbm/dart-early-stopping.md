LightGBM's early stopping callback disables itself in DART mode. This also
applies when `train()` or `cv()` creates the callback from a positive
`early_stopping_round` parameter or one of its aliases.

```diff
-lgb.train({"boosting": "dart", "early_stopping_round": 10}, train_set, valid_sets=[valid_set])
+lgb.train({"boosting": "gbdt", "early_stopping_round": 10}, train_set, valid_sets=[valid_set])
```

## How to fix it

Choose GBDT if you need LightGBM's early stopping. To keep DART, choose an
iteration count using separate validation.

The rule checks literal native parameters and explicit LightGBM
`early_stopping()` calls with positive literal patience in a callback list.

[Early stopping API](https://lightgbm.readthedocs.io/en/v4.7.0/pythonapi/lightgbm.early_stopping.html) |
[Training engine](https://lightgbm.readthedocs.io/en/v4.7.0/_modules/lightgbm/engine.html)
