Conformalizing on the training rows makes the model's errors look smaller than on new rows, so the intervals cover about half of what you asked for. Conformalize on rows the fit never saw.

```diff lang="python"
+X_train, X_conf, X_test, y_train, y_conf, y_test = train_conformalize_test_split(
+    X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2, random_state=0
+)
 model.fit(X_train, y_train)
 mapie = SplitConformalRegressor(model, confidence_level=0.9, prefit=True)
-mapie.conformalize(X_train, y_train)
+mapie.conformalize(X_conf, y_conf)
```

## What the rule reports

The rule reports `conformalize` on the training rows, known by the name of its first argument or
because an earlier `fit` got the same variable. It also reports `calibrate` of a risk controller on
the training rows, and `fit` of a `TopLabelCalibrator` or `VennAbersCalibrator` with
`cv="prefit"` on the training rows.

## How to fix it

xpd has no automatic fix. The rows held out from the fit are not in the file. Make the change by hand.

| Form | Old form | Right form |
| --- | --- | --- |
| a regressor conformalized on its training rows | 0.468 coverage | 0.907 with held-out rows |
| a `RandomForestClassifier` conformalized on its training rows | 0.467 coverage | 0.906 |
| a model fitted on rows that include the conformalization rows | 0.473 coverage | not given |
| a `GridSearchCV` fitted on the conformalization rows | 0.679 coverage | 0.902 |
| a `BinaryClassificationController` with a target precision of 0.85, calibrated on the training rows | precision 0.638 on test rows, below the target in 30 of 30 runs | with held-out rows, a threshold in 1 of 30 runs and none below the target |
| a `TopLabelCalibrator` with `cv="prefit"` fitted on the training rows | top-label calibration error 0.178 | 0.085 with held-out rows, and 0.124 without calibration |
| a `VennAbersCalibrator` fitted on the training rows | top-label calibration error 0.128 | 0.061 |

The rule reads the name for `train`, `trn`, `tr`, `fit` and `training`, and leaves alone a name
with `conf`, `calib`, `cal`, `val`, `hold`, `test` or `res` in it. The form without a name needs an
earlier `fit` with a target in the same block or the same chain. A scaler fitted on the
conformalization rows is therefore left alone, and so is a class that calls `fit` and
`conformalize` on itself. A `fit` is left out when a statement between it and the `conformalize`
assigns to the variable, as in `X, y = load("conformalize")`, because the name then holds other
rows.

A model picked from six settings by its score on the conformalization rows covered 0.901, against
0.904 for a model picked by cross-validation on the training rows, so the rule doesn't report it.
