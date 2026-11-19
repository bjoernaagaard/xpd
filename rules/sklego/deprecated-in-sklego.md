These names, keywords and attributes warn on scikit-lego and will be removed. Use the replacement that the message names.

```diff lang="python"
-folds = KlusterFoldValidation(cluster_method=KMeans(3))
-splitter = TimeGapSplit(date_serie=dates, train_duration=timedelta(days=10), valid_duration=timedelta(days=5))
-width = capper.n_columns_
+folds = ClusterFoldValidation(cluster_method=KMeans(3))
+splitter = TimeGapSplit(date_series=dates, train_duration=timedelta(days=10), valid_duration=timedelta(days=5))
+width = capper.n_features_in_
```

## What the rule reports

The rule reports the names, keywords and fitted attributes that raise a DeprecationWarning:

| Old form | Write instead |
| --- | --- |
| `KlusterFoldValidation` | `ClusterFoldValidation` |
| `PandasTypeSelector` | `TypeSelector` |
| `TimeGapSplit(date_serie=)` | `date_series=` |
| the attributes `dim_`, `num_fit_cols_`, `n_columns_` and `num_cols_` | `n_features_in_` |
| `n_levels_` | `n_fitted_levels_` |
| `coefs_` | `coef_` |
| `penalty="none"` on the fairness classifiers | `penalty=None` |
| the properties `allowed_strategies`, `allowed_methods` and `allowed_effects` | no fix |

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `ClusterFoldValidation` for `KlusterFoldValidation`, `TypeSelector` for `PandasTypeSelector`, `date_series=` for `date_serie=`, `n_features_in_` for `dim_`, `num_fit_cols_`, `n_columns_` and `num_cols_`, `n_fitted_levels_` for `n_levels_`, `coef_` for `coefs_`, and `penalty=None` for `penalty="none"`. `allowed_strategies`, `allowed_methods` and `allowed_effects` have no fix.

Every fix was run before and after on the pack's sample file, and the folds, the selected columns,
the attribute values, the coefficients and the predictions were equal.

An old attribute name is reported only on an object that the file builds from a scikit-lego class
with that property. The object is a call chain that starts at the class, as in
`ColumnCapper().fit(X).n_columns_`, or a name that the file assigns such a chain to:

| Attribute | Classes |
| --- | --- |
| `dim_` | `RandomRegressor`, `RandomAdder`, `DictMapper`, `OutlierRemover` |
| `num_fit_cols_` | the two mixture naive Bayes classifiers |
| `n_columns_` | `ColumnCapper` |
| `num_cols_` | `IntervalEncoder` |
| `n_levels_` | `HierarchicalRegressor`, `HierarchicalClassifier` |

`coefs_` is reported only in a file that names `ProbWeightRegression` or `DeadZoneRegressor` and no
MLP, because scikit-learn's MLP models have their own `coefs_`. The same name on any other object,
such as `cfg.dim_` on your own class, is left alone, because the rename would raise AttributeError
there. An assignment to the name, and the definition of a function or class of the name, are the
file's own and are left alone. Each document carries `since` with the release that began to warn.
