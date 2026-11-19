The MAPIE 0.x API was removed, so the import or the call raises on MAPIE 1.6. Use the 1.x class that the message names.

```diff lang="python"
-from mapie.regression import MapieRegressor
-from mapie.metrics import regression_coverage_score
+from mapie.regression import SplitConformalRegressor
+from mapie.metrics.regression import regression_coverage_score
 
-mapie = MapieRegressor(model, cv="prefit")
-mapie.fit(X_conf, y_conf)
-y_pred, y_pis = mapie.predict(X_test, alpha=0.1)
-coverage = regression_coverage_score(y_test, y_pis[:, 0, 0], y_pis[:, 1, 0])
+mapie = SplitConformalRegressor(model, confidence_level=0.9, prefit=True)
+mapie.conformalize(X_conf, y_conf)
+y_pred, y_pis = mapie.predict_interval(X_test)
+coverage = regression_coverage_score(y_test, y_pis)[0]
```

## What the rule reports

The rule reports the MAPIE 0.x API that raises on MAPIE 1.6.0:

| Old form | Where the rule reports it |
| --- | --- |
| `MapieRegressor`, `MapieClassifier`, `MapieQuantileRegressor`, `MapieTimeSeriesRegressor`, `MapieCalibrator`, `MapieMultiLabelClassifier`, `PrecisionRecallController`, `MondrianCP`, `ConformityScore` | an import |
| modules that are gone, such as `mapie.quantile_regression`, `mapie.control_risk` and `mapie._typing` | an import |
| metric functions from `mapie.metrics` itself | an import |
| `alpha=` | the MAPIE 1 constructors, `predict_interval`, `predict_set`, `predict` of an object that the file builds from a MAPIE 1 class, and the metrics |
| `fit` | a `CrossConformalRegressor`, `CrossConformalClassifier` or `JackknifeAfterBootstrapRegressor` |
| `cv=` | the split classes |
| `cv="prefit"` or `cv="split"` | the cross classes |
| `method=` | the classifiers and `TopLabelCalibrator` |
| `method="naive"` | any class |
| `sample_weight=` | `fit` or `fit_conformalize` |
| `partial_fit` | a `TimeSeriesRegressor` |
| the lower and upper bounds as two arguments | a metric |

The rule leaves alone an import inside a `try` block that catches the error, code behind a version
check, and code inside `pytest.raises`. Each of its documents carries the release that removed the
form:

| Form | Removed in |
| --- | --- |
| most forms | 1.0 |
| `partial_fit` | 1.2 |
| `PrecisionRecallController` | 1.3 |
| `sample_weight=` on `TimeSeriesRegressor.fit` | 1.4.1 |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `numpy.typing` for `mapie._typing`, because on MAPIE 0.9.2 `ArrayLike` and `NDArray` of `mapie._typing` are the objects of `numpy.typing`. The unsafe fix writes `mapie.metrics.regression`, `mapie.metrics.classification` or `mapie.metrics.calibration` for `mapie.metrics` when every imported name is in that module, `confidence_level=1 - alpha` for `alpha=` on a constructor, on `TimeSeriesRegressor.predict`, on `regression_mwi_score` and on `coverage_width_based` when the value is a number or a name, `fit_params={"sample_weight": w}` for `sample_weight=w`, `update` for `partial_fit`, and one stacked array for the two bounds given to `regression_coverage_score` and `regression_mean_width_score`. A list of numbers given as `alpha=` on a constructor or on `TimeSeriesRegressor.predict` becomes `confidence_level=` with 1 minus each number, `fit_conformalize` replaces `fit` on a cross or jackknife object, `method=` becomes `conformity_score=` on a classifier when its value is `"lac"`, `"aps"`, `"raps"` or `"top_k"`, and `method=` is taken out of `TopLabelCalibrator`, which drops the comments between the arguments. The removed classes and modules, `alpha=` on `predict_interval`, `predict_set` and `predict`, `cv=` and the other values of `method` have no fix, because the rewrite reaches other calls or the message names more than one. No release ran an unsafe fix's old and new form side by side, except `sample_weight=` and `fit_params=` on `TimeSeriesRegressor.fit` in 1.4.0, which ignored `sample_weight=` and gave other values, and `partial_fit` and `update` in 1.1.0, which are two methods.

The forms around `partial_fit` and `sample_weight` changed in the following releases:

| Form | Worked on | Failed on |
| --- | --- | --- |
| `TimeSeriesRegressor.partial_fit` | 1.1.0, in the wheel | 1.2.0, where it is gone |
| `sample_weight=` on `TimeSeriesRegressor.fit` | up to 1.4.0 | 1.4.1 and later, with TypeError |
| `fit_params={"sample_weight": w}` | 1.4.0 and later | 1.0.1 to 1.3.0 |
| `sample_weight=` on `fit` of the split classes | not given | 1.0.1 already |

The fix `confidence_level=1 - alpha` is written only for a number or a name. `alpha=[0.05, 0.2]`
was common in 0.x code, and `1 - [0.05, 0.2]` raises TypeError, so a list of numbers gets
`confidence_level=[1 - 0.05, 1 - 0.2]`. Another expression is reported without a fix, because the
file does not show whether it is one number or a list. `confidence_level=[0.95, 0.8]` gave
intervals of shape (3, 2, 2) for 3 rows.

`predict(X, alpha=0.1)` raised TypeError on each of the split, cross, jackknife and quantile
classes. Their level is set in the constructor, so the `predict` document has no fix. `fit` raised
AttributeError on `CrossConformalRegressor`, `CrossConformalClassifier` and
`JackknifeAfterBootstrapRegressor`, on 1.0.1 and on 1.6.0. The rule reports it on a name that the
file assigns from one of the three and from nothing else. `CrossConformalizedQuantileRegressor` has
a `fit` on 1.6.0 and is left alone.

The rule reports an import and doesn't rename the class, because the constructors differ.
`MapieRegressor(model, cv="prefit")` becomes `SplitConformalRegressor(model, prefit=True)`, and
`alpha` moves from `predict` to the constructor. Two fixes write `np.` and apply only in a file
with `import numpy as np`. On the same bounds, the fixed calls of `regression_coverage_score`,
`regression_mean_width_score` and `coverage_width_based` on MAPIE 1.6.0 returned the value that
the old calls returned on MAPIE 0.9.2, bit for bit, and `regression_mwi_score` returned it to 15
digits.
