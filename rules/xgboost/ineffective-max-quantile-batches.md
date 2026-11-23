XGBoost 3.3 deprecated `max_quantile_batches` and made it ineffective.
Setting it no longer limits how many GPU batches XGBoost combines when
building a quantile sketch.

```diff
-train = xgb.QuantileDMatrix(features, label=labels, max_quantile_batches=4)
+train = xgb.QuantileDMatrix(features, label=labels)
```

## What the rule reports

The rule reports explicit values other than literal `None` on
`QuantileDMatrix` and `ExtMemQuantileDMatrix` constructors from XGBoost.

## How to fix it

Remove the keyword from code that targets XGBoost 3.3 or later. If your
workload depended on the old limit, recheck its peak memory use.

[QuantileDMatrix API](https://xgboost.readthedocs.io/en/stable/python/python_api.html#xgboost.QuantileDMatrix) · [ExtMemQuantileDMatrix API](https://xgboost.readthedocs.io/en/stable/python/python_api.html#xgboost.ExtMemQuantileDMatrix)
