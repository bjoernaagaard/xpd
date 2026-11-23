These modules, names and arguments were removed from scikit-learn, so the code raises. Use the replacement that the message names.

```diff lang="python"
-from sklearn.cross_validation import train_test_split
+from sklearn.model_selection import train_test_split
 
-encoder = OneHotEncoder(sparse=False)
-names = vectorizer.get_feature_names()
-forest = RandomForestClassifier(max_features="auto", random_state=0)
-folds = StratifiedKFold(y, n_folds=5)
+encoder = OneHotEncoder(sparse_output=False)
+names = vectorizer.get_feature_names_out()
+forest = RandomForestClassifier(max_features="sqrt", random_state=0)
+folds = StratifiedKFold(n_splits=5).split(X, y)
```

## What the rule reports

The rule reports imports of removed modules and names, such as `sklearn.cross_validation`,
`sklearn.externals.joblib` and `six`, `load_boston`, `preprocessing.Imputer`, private module paths
and `sklearn.feature_extraction.stop_words`. It also reports the following forms:

- `get_feature_names()`
- keywords that the signature no longer has
- values that parameter validation refuses
- positional arguments past the keyword-only marker
- splitters given the data the way `sklearn.cross_validation` took it

## How to fix it

The unsafe fixes for `multi_class="ovr"` and `cv="prefit"` can import their wrappers when the names are available.

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `from joblib import` for the names that `sklearn.externals.joblib` took from joblib, the public module for a private module path such as `sklearn.linear_model.logistic`, `sklearn.feature_extraction.text` for `stop_words`, the renamed keywords `sparse_output=`, `estimator=`, `max_iter=`, `store_cv_results=`, `ensure_all_finite=`, `metric=` and `alphas=`, dropping `n_jobs` from `KMeans`, `behaviour="new"` and `"deprecated"`, `algorithm="SAMME"`, `iid=False`, `multi_class="auto"`, `min_impurity_split=None`, and `normalize=False` outside the Lars and orthogonal matching pursuit models, new values for refused ones, such as `max_features="sqrt"` or `1.0` for `"auto"`, `loss="log_loss"`, `criterion="squared_error"` and `"absolute_error"`, `loss="squared_error"`, `C=np.inf` for `penalty="none"` and `min_samples_split=2` for `1`, `max_iter=` with `tol=None` for `n_iter=` on the SGD models, and `alphas=` for `n_alphas=` beside `alphas=None`. unsafe, because no older release ran both forms with the same result: `sklearn.model_selection` for `sklearn.cross_validation`, `sklearn.grid_search` and `sklearn.learning_curve`, `import joblib` and `import joblib as` for the module `sklearn.externals.joblib`, a private module path that scikit-learn 0.19.2 to 0.23.2 did not have, or that imports `VALID_METRICS` from a neighbors module or `all_estimators` from `sklearn.utils.testing`, `get_feature_names_out()`, which returns an array, `n_splits=` for `n_folds=` and `n_iter=`, which the `model_selection` splitters never took, dropping `n_jobs` from `MiniBatchKMeans`, which never took it, dropping the other values of `behaviour`, `algorithm` and `iid`, `multi_class="multinomial"`, `normalize=False` on the Lars and orthogonal matching pursuit models, `OneVsRestClassifier(LogisticRegression())` for `multi_class="ovr"`, and `CalibratedClassifierCV(FrozenEstimator(model))` for `cv="prefit"`. A fix that rewrites the whole call applies only when the keyword is on the first line of the call. The other forms have no fix.

The proof ran 49 forms on scikit-learn 1.9.1. Each old form raised ModuleNotFoundError,
ImportError, AttributeError, TypeError for an unexpected keyword or too many positional arguments,
or InvalidParameterError, and its new form ran clean. A grid with
`max_features: ["auto", "sqrt"]` raised through the FitFailedWarning that `GridSearchCV` gives. A
backslash line break inside a call is not counted as a positional argument. `SCORERS`, the
`plot_*` functions, `jaccard_similarity_score`, `squared=` and `fit_params=` are left to
[`removed-metric-api`](removed-metric-api.md) and [`fit-params-keyword`](fit-params-keyword.md).

Every fixed form ran on the case built for it, and each renamed keyword and refused value gave the
same model as a reference written from the 1.9.1 docstrings. Some fixes change what the code
means:

| Fix | What changes |
| --- | --- |
| `get_feature_names_out()` | It returns an array where `get_feature_names()` returned a list, so `print` shows `array([...])`. At 1 held-back site the fix broke a project class that defined its own `get_feature_names`. |
| dropping `multi_class` | Every multiclass fit uses the multinomial loss. With liblinear on three or more classes the fixed call raises ValueError. |
| `OneVsRestClassifier(...)` for `multi_class="ovr"` | The model is a `OneVsRestClassifier`, so `coef_` moves to its `estimators_`, and a grid key such as `logisticregression__C` needs `estimator__` in front. |
| dropping `algorithm="SAMME.R"` | A SAMME.R model becomes a SAMME model. |
| `OneHotEncoder(sparse_output=False, dtype=np.int)` | At 2 held-back sites the call also held `np.int`, which is gone from NumPy. The same run with `--fix --unsafe-fixes` rewrote it to `int`, and the fixed call ran. |

Of 321 held-back reports, 58 were labelled, and reader A called 1 wrong. Recall was 517 of 600.
Of the misses, 36 were `SCORERS`, the `plot_*` functions and `jaccard_similarity_score`, which
`removed-metric-api` reports. The rule found none of 18 refused values in a parameter grid for
keywords other than `max_features`, and none of 16 refused values held in a variable. It also
misses `sklearn.feature_extraction.stop_words` imported as a module.
