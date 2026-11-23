XGBoost's `EarlyStopping` callback stores training history and a count of
rounds without improvement. Reusing it can carry those values into a new fit
and stop that fit sooner than intended.

```diff
-stop = xgb.callback.EarlyStopping(rounds=5)
 for params in parameter_grid:
+    stop = xgb.callback.EarlyStopping(rounds=5)
     model = xgb.train(params, train, evals=[(valid, "valid")], callbacks=[stop])
```

## What the rule reports

The rule checks `train` calls in a loop that pass an `EarlyStopping`
instance created immediately before that loop as the only callback.

## How to fix it

Construct a fresh callback for each training run. Where a callback must be
copied, copy the unused instance before any fit.

[Training API and callback reuse](https://xgboost.readthedocs.io/en/stable/python/python_api.html#xgboost.train)
