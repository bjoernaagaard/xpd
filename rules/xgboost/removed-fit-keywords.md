XGBoost 2.1 removed `eval_metric`, `early_stopping_rounds` and `callbacks`
from the scikit-learn estimator's `fit` method. Passing any of them there
raises `TypeError`.

```diff
-model = xgb.XGBRegressor()
-model.fit(X_train, y_train, eval_set=[(X_valid, y_valid)], early_stopping_rounds=5)
+model = xgb.XGBRegressor(early_stopping_rounds=5)
+model.fit(X_train, y_train, eval_set=[(X_valid, y_valid)])
```

## What the rule reports

The rule follows an estimator to a constructor imported from XGBoost and
checks explicit keywords on its `fit` call.

## How to fix it

Move the options to the constructor or call `set_params` before fitting.
Keep `eval_set` on `fit`. When the same estimator is fitted again, check
which settings each call should use.

[2.1 release notes](https://xgboost.readthedocs.io/en/stable/changes/v2.1.0.html#breaking-changes)
