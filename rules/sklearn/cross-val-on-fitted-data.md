A step that learned from the target on every row before cross-validation lets each fold be scored on rows whose labels chose the features. Put the step and the model in one Pipeline and cross-validate the Pipeline.

```diff lang="python"
-X_picked = selector.fit_transform(X, y)
-scores = cross_val_score(model, X_picked, y, cv=5)
+pipeline = make_pipeline(selector, model)
+scores = cross_val_score(pipeline, X, y, cv=5)
```

## What the rule reports

The rule reports a step that learns from the target, or `fit_resample`, run on every row before
`cross_val_score`, another cross-validation function, or a search fitted on the result.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The rule covers `cross_val_score`, `cross_validate`, `cross_val_predict`, `learning_curve`,
`validation_curve` and `permutation_test_score` when the fitted result is the second argument. It
also covers a `GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV` or
`HalvingRandomSearchCV` that is assigned in the same file and fitted on the result.

| Step fitted on every row | Cross-validated AUC | With the step inside the folds |
| --- | --- | --- |
| `SelectKBest`, then `cross_val_score` | 0.764 | 0.518 for a Pipeline |
| `SMOTE`, then `cross_val_score` | 0.838 | 0.510 |
| `SelectKBest`, then a search, as `best_score_` | 0.770 | 0.522 |
| `SMOTE`, then a search, as `best_score_` | 0.854 | 0.521 |
| `RandomUnderSampler`, left alone | 0.519 | 0.511 |
| `NearMiss`, left alone | 0.458 | 0.508 |
| `TomekLinks`, left alone | 0.530 | 0.508 |

The three under-samplers didn't pass the proof, so the rule leaves them alone. The rule as the
readers saw it gave wrong advice in 1 and 0 of 11 labelled reports.
