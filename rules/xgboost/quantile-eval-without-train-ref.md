XGBoost `train` requires a separate `QuantileDMatrix` or
`ExtMemQuantileDMatrix` used for evaluation to reference the exact matrix
passed as `dtrain`. The reference makes the two matrices use
the same quantile cuts.

```diff
 train = xgb.QuantileDMatrix(X_train, label=y_train)
-valid = xgb.QuantileDMatrix(X_valid, label=y_valid)
+valid = xgb.QuantileDMatrix(X_valid, label=y_valid, ref=train)
 model = xgb.train({}, train, evals=[(train, "train"), (valid, "valid")])
```

## What the rule reports

The rule checks literal `evals` lists and tuples passed to `train`. It
checks evaluation matrices constructed inline or directly before training
with `ref` omitted or set to `None`. It also reports a reference when
separate constructor calls establish that `ref` and `dtrain` are different
objects. The training matrix itself can appear in the evaluation list.

## How to fix it

Pass the training matrix as `ref` when you construct the evaluation matrix.
If you construct `dtrain` inside the call, assign it to a name first and
reuse that name for both arguments. You can also use a regular `DMatrix` for evaluation.

[QuantileDMatrix API](https://xgboost.readthedocs.io/en/stable/python/python_api.html#xgboost.QuantileDMatrix) · [XGBoost 3.4.2 train implementation](https://github.com/dmlc/xgboost/blob/v3.4.2/python-package/xgboost/training.py#L158-L181)
