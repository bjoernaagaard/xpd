Building one conformal object per confidence level fits the model once per level. Pass the list of levels as `confidence_level` and fit once.

```diff lang="python"
-for level in (0.8, 0.9, 0.95):
-    cross = CrossConformalRegressor(model, confidence_level=level, cv=5, random_state=0)
-    cross.fit_conformalize(X_train, y_train)
-    intervals[level] = cross.predict_interval(X_test)[1][:, :, 0]
+levels = [0.8, 0.9, 0.95]
+cross = CrossConformalRegressor(model, confidence_level=levels, cv=5, random_state=0)
+cross.fit_conformalize(X_train, y_train)
+_, y_pis = cross.predict_interval(X_test)
+intervals = {level: y_pis[:, :, i] for i, level in enumerate(levels)}
```

## What the rule reports

The rule reports a conformal object built inside a loop or a comprehension with the loop variable
as its `confidence_level`.

## How to fix it

xpd has no automatic fix. The rewrite replaces the loop around the call. Make the change by hand.

`confidence_level` takes a list, and the intervals then have one column per level. With three
levels, one seeded object per level took longer than one object for the list, and the intervals
were bit equal:

| Class | At 1,000 rows | At 5,000 rows |
| --- | --- | --- |
| `CrossConformalRegressor` | 2.87 times | 2.95 times |
| `SplitConformalRegressor` | 2.99 times | 3.01 times |

Without `random_state` a cross class draws other folds for each object, so the intervals per level
then differ from the list form. `ConformalizedQuantileRegressor` takes one level only, and the rule
leaves it alone.
