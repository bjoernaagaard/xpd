LightGBM 4.0 removed the `early_stopping_rounds`, `verbose_eval` and `fobj`
keywords from `train()` and `cv()`, and `evals_result` from `train()`.
It also removed `early_stopping_rounds` and `verbose` from estimator `fit()`.
LightGBM 4.6 removed `feature_name` and `categorical_feature` from
`train()` and `cv()`.

```diff
-lgb.train(params, train_set, valid_sets=[valid_set], early_stopping_rounds=10)
+lgb.train(params, train_set, valid_sets=[valid_set], callbacks=[lgb.early_stopping(10)])
```

## How to fix it

Use callbacks for early stopping, periodic logging and evaluation history.
Keep any existing callbacks when adding `early_stopping()`,
`log_evaluation()` or `record_evaluation(history)`. For a custom objective,
pass the callable as `params["objective"]`. Configure feature names and
categorical features on the `Dataset`.

`early_stopping_rounds` is still accepted as an alias for the
`early_stopping_round` setting inside `params`.

On a current API, the unsafe fix removes the literal no-op keywords
`fobj=None` and `verbose_eval=False` from native calls and `verbose=False`
from estimator `fit()`. Migrate the
remaining calls by hand to preserve the intended stopping criterion,
logging, evaluation history, objective or dataset metadata. Omit estimator
`verbose=False` when you want no evaluation logging. To enable evaluation
logging, add `log_evaluation()` to the callbacks.

[4.0 release notes](https://github.com/lightgbm-org/LightGBM/releases/tag/v4.0.0) |
[4.6 release notes](https://github.com/lightgbm-org/LightGBM/releases/tag/v4.6.0) |
[Parameter reference](https://lightgbm.readthedocs.io/en/v4.7.0/Parameters.html#early_stopping_round)
