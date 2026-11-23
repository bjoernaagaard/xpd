XGBoost's native `Booster.predict` and `Booster.inplace_predict` use all
retained rounds by default, including any rounds after early stopping found
the best validation score. The scikit-learn estimators use the best round
automatically.

```diff
 model = xgb.train(params, train, evals=[(valid, "valid")], early_stopping_rounds=5)
-predictions = model.predict(test)
+predictions = model.predict(test, iteration_range=(0, model.best_iteration + 1))
```

## What the rule reports

The rule follows a native booster from a `train` call with a positive integer
literal for `early_stopping_rounds`. It checks prediction calls that omit
`iteration_range`. Explicit training callbacks and unpacked arguments can
change the behavior and are outside this check.

## How to fix it

Pass `iteration_range=(0, model.best_iteration + 1)` for best-round
predictions. The upper end is exclusive. For tree boosters,
`EarlyStopping(save_best=True)` can discard later rounds during training.
Use `iteration_range=(0, 0)` to request all retained rounds explicitly.

[Early stopping and prediction](https://xgboost.readthedocs.io/en/stable/prediction.html#early-stopping)
