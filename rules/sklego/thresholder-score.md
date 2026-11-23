`Thresholder.score` ignores the threshold, so a search over thresholds scores every one the same. Pass `scoring=`, or use scikit-learn's `FixedThresholdClassifier`.

```diff lang="python"
-search = GridSearchCV(Thresholder(LogisticRegression(), threshold=0.5), {"threshold": [0.1, 0.5, 0.9]})
-accuracy = Thresholder(LogisticRegression(), threshold=0.1).fit(X_train, y_train).score(X_test, y_test)
+search = GridSearchCV(Thresholder(LogisticRegression(), threshold=0.5), {"threshold": [0.1, 0.5, 0.9]}, scoring="accuracy")
+model = FixedThresholdClassifier(LogisticRegression(), threshold=0.1, response_method="predict_proba")
+accuracy = model.fit(X_train, y_train).score(X_test, y_test)
```

## What the rule reports

The rule reports `GridSearchCV`, `RandomizedSearchCV`, `cross_val_score`, `cross_validate` and the
other search and validation functions when they hold a `Thresholder(...)` and no `scoring=`. It
also reports `.score(...)` on a `Thresholder`. `Thresholder.score` returns the score of the inner
model, so it doesn't change with `threshold`.

## How to fix it

With `--unsafe-fixes`, xpd adds `scoring="accuracy"` to matching search and cross validation calls.

On 400 rows with 20 percent positives, from `make_classification(n_samples=400, weights=[0.8],
random_state=0)` and `train_test_split(random_state=0)`, the grid search on the training rows gave
its mean test score, and `score` and `predict` ran on the test rows:

| Threshold | Grid search without `scoring` | With `scoring="accuracy"` | `Thresholder.score` | Accuracy of `predict` |
| --- | --- | --- | --- | --- |
| 0.1 | 0.887 | 0.740 | 0.850 | 0.710 |
| 0.5 | 0.887 | 0.887 | 0.850 | 0.850 |
| 0.9 | 0.887 | 0.847 | 0.850 | 0.840 |

scikit-learn's `FixedThresholdClassifier` made the same predictions as `Thresholder` at each
threshold, and its `score` gave 0.710, 0.850 and 0.840.
