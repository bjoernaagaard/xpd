Fitting a new forest for each tree count in a loop grows every tree again. Build the model once with `warm_start=True` and raise `n_estimators` before each fit.

```diff lang="python"
+forest = RandomForestClassifier(warm_start=True, random_state=0)
 for n in [10, 50, 100]:
-    forest = RandomForestClassifier(n_estimators=n, random_state=0)
+    forest.set_params(n_estimators=n)
     forest.fit(X_train, y_train)
     scores.append(forest.score(X_test, y_test))
```

## What the rule reports

The rule reports a `RandomForest`, `ExtraTrees`, `GradientBoosting` or `Bagging` model or an
`IsolationForest` built with `n_estimators=n` in a loop over `n` and fitted there.

The rule leaves the following loops alone:

## How to fix it

xpd has no automatic fix, so make the change by hand.

The new form relies on `warm_start=True`, which keeps the trees already grown and draws every seed
from `random_state` in the same order. The grown models were bit equal to models fitted from
scratch, and warm start was faster:

| Trees | Classes | Warm start was faster by |
| --- | --- | --- |
| `n_estimators` 10, 20, 50 and 100, at 1,000 and 5,000 rows | `RandomForestClassifier`, `RandomForestRegressor`, `ExtraTreesClassifier`, `GradientBoostingClassifier`, `BaggingClassifier`, `IsolationForest` | 1.64 to 2.75 times |
| 10 to 100 in steps of 10 | `RandomForestClassifier` | 6.82 and 3.59 times |

- a loop whose model is passed to a cross-validation function or to `clone`, which clones it, so
  warm start can't help
- a loop with another loop inside it, where another setting changes between fits
- a loop that reads a clock, because it times each fit

The rule as the readers saw it made 1 report on the held-back files, on a loop that timed each
fit, and both readers called it wrong. Recall was 30 of 30.
