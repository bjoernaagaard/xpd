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

Use `xpd check --fix` to remove an integer literal setting that follows
another argument in a call without unpacked arguments or comments. Remove
other settings by hand so you can preserve any evaluation of their values.
If your
workload depended on the old limit, recheck its peak memory use.

[QuantileDMatrix API](https://xgboost.readthedocs.io/en/stable/python/python_api.html#xgboost.QuantileDMatrix) · [ExtMemQuantileDMatrix API](https://xgboost.readthedocs.io/en/stable/python/python_api.html#xgboost.ExtMemQuantileDMatrix)
