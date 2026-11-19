`fit_params=` was removed and raises TypeError on scikit-learn 1.9. Write `params=` instead.

```diff lang="python"
-scores = cross_val_score(model, X, y, fit_params={"sample_weight": w})
+scores = cross_val_score(model, X, y, params={"sample_weight": w})
```

## What the rule reports

The rule reports `fit_params=` on `cross_val_score`, `cross_validate`, `cross_val_predict`,
`learning_curve`, `validation_curve`, `permutation_test_score` and the four search classes.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `params=` for `fit_params=` on the cross-validation functions, because the old form raises. The search form has no fix.

xpd applies the replacement with `--fix --unsafe-fixes`.

The keyword raises TypeError on scikit-learn 1.9.1. The documents for `cross_val_score`,
`cross_validate` and `cross_val_predict` carry `since: "1.4"`, the release that added `params`,
and the documents for the other three functions carry `since: "1.6"`. On `GridSearchCV`,
`RandomizedSearchCV`, `HalvingGridSearchCV` and `HalvingRandomSearchCV` the message says to drop
the keyword and pass the values to `fit`. The rename is an unsafe fix, because the old form raises
and leaves nothing to compare. Neither reader called one of the 4 held-back reports wrong. Recall was
32 of 32.
