XGBoost 3.1 removed the `gpu_hist` tree method. Training a tree booster with
that value raises an error.

```diff
-model = xgb.XGBClassifier(tree_method="gpu_hist")
+model = xgb.XGBClassifier(tree_method="hist", device="cuda")
```

## What the rule reports

The rule checks literal `gpu_hist` values on XGBoost estimator constructors
and in dictionaries passed directly as `params` to `train` or `cv`. For
duplicate keys, the last value takes effect. Raw and Unicode string prefixes
are accepted on literal keys and values.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to migrate an estimator constructor
whose only argument is `tree_method="gpu_hist"`. For other cases, edit the
configuration by hand. Use `tree_method="hist"` with `device="cuda"` for GPU training. If the old
configuration selected a specific GPU with `gpu_id`, use the corresponding
device string, such as `device="cuda:1"`, in single-node training.

[3.1 release notes](https://xgboost.readthedocs.io/en/stable/changes/v3.1.0.html#removing-deprecated-features) · [Device and tree method parameters](https://xgboost.readthedocs.io/en/stable/parameter.html)
