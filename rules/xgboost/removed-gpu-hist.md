XGBoost 3.1 removed the `gpu_hist` tree method. Training a tree booster with
that value raises an error.

```diff
-model = xgb.XGBClassifier(tree_method="gpu_hist")
+model = xgb.XGBClassifier(tree_method="hist", device="cuda")
```

## What the rule reports

The rule checks literal `gpu_hist` values on XGBoost estimator constructors
and in dictionaries passed directly as `params` to `train` or `cv`.

## How to fix it

Use `tree_method="hist"` with `device="cuda"` for GPU training. If the old
configuration selected a specific GPU with `gpu_id`, use the corresponding
device string, such as `device="cuda:1"`, in single-node training.

[3.1 release notes](https://xgboost.readthedocs.io/en/stable/changes/v3.1.0.html#removing-deprecated-features) · [Device and tree method parameters](https://xgboost.readthedocs.io/en/stable/parameter.html)
