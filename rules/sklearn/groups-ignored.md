The chosen `cv` ignores `groups=`, so rows of one group land in both the training and the test folds, and the score is too high. Use a group splitter such as `GroupKFold`.

```diff lang="python"
-scores = cross_val_score(model, X, y, groups=patient_id, cv=5)
+scores = cross_val_score(model, X, y, groups=patient_id, cv=GroupKFold(n_splits=5))
```

## What the rule reports

The rule reports `groups=` given to a cross-validation function or a search whose `cv` ignores
it. It also reports the groups given to `split` on a splitter that ignores them, such as
`KFold(5).split(X, y, g)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `cv=GroupKFold(5)` for `cv=KFold(5)`, `cv=GroupKFold()` when cv is missing, `cv=GroupKFold(5)` for `cv=5`, and `cv=GroupKFold(n_splits=5)` in a search built inside the fit call. The folds change on purpose. Other splitters, a splitter or a search held in a name, and a call to split have no fix.

xpd applies the replacement with `--fix --unsafe-fixes`.

scikit-learn warns "The groups parameter is ignored by KFold" and splits the rows as if no groups
were given. How much the score leaks depends on the row order:

| Data | Model | `cv=5` with `groups=` | `GroupKFold(5)` |
| --- | --- | --- | --- |
| 60 groups of 8 rows in random order, one label per group drawn apart from the features | `RandomForestClassifier` | 0.849 accuracy | 0.480 |
| the same groups with the rows of each group next to each other, and folds that are not shuffled | `RandomForestClassifier` | 0.497 | not given |
| 40 groups of 5 near copies | 1-nearest-neighbour | 0.998 | 0.487 |

A `cv` ignores groups when it is missing, a whole number, or one of `KFold`, `StratifiedKFold`,
`ShuffleSplit`, `StratifiedShuffleSplit`, `RepeatedKFold`, `RepeatedStratifiedKFold`,
`TimeSeriesSplit`, `LeaveOneOut`, `LeavePOut` and `PredefinedSplit`, given in the call or assigned
to a name earlier. The rule reports such a `cv` in the following places:

- as `cv` of `cross_val_score`, `cross_validate`, `cross_val_predict`, `learning_curve`,
  `validation_curve` and `permutation_test_score` with `groups=`
- in a `GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV` or `HalvingRandomSearchCV` built
  in the same block as a `fit` that gets `groups=`
- in its own `split(X, y, groups)`, called directly or through a name

A `cv` held in a name that xpd can't trace to one of the listed splitters is left alone. Recall was 56
of 57.
