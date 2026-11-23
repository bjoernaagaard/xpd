XGBoost's `Booster.inplace_predict` can score supported batches for tree boosters without
allocating a new `DMatrix` for each batch.

```diff
 for batch in batches:
-    yield model.predict(xgb.DMatrix(batch))
+    yield model.inplace_predict(batch)
```

## What the rule reports

The rule checks a native booster that calls `predict(DMatrix(batch))` in
the body of the loop that binds `batch`. It looks for a single positional
argument in both calls and skips boosters with a known `gblinear` setting.
It recognizes a literal parameter dictionary passed inline or assigned to
the argument name directly before the booster assignment.

## How to fix it

Use `inplace_predict` for a tree booster when the input type and device match
what it supports and you only need ordinary predictions. For `gblinear`, keep
`DMatrix` because in-place prediction raises `XGBoostError`. Keep `DMatrix` for inputs
or prediction features that require it. When scoring the same matrix
repeatedly, you can reuse it and benefit from native prediction caching.

[In-place and staged prediction](https://xgboost.readthedocs.io/en/stable/prediction.html#in-place-prediction)
