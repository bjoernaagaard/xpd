A group splitter without `groups=` raises ValueError. Pass `groups=` with the group of each row.

```diff lang="python"
 search = GridSearchCV(model, grid, cv=GroupKFold(n_splits=5))
-search.fit(X, y)
+search.fit(X, y, groups=patient_id)
```

## What the rule reports

The rule reports a group splitter that gets no groups, such as
`cross_val_score(model, X, y, cv=GroupKFold(5))` or `GroupKFold(5).split(X, y)`, which raises
ValueError.

## How to fix it

xpd has no automatic fix, so make the change by hand.

On scikit-learn 1.9.1 each form raises "The 'groups' parameter should not be None", and
`StratifiedGroupKFold` raises that it found 1 group. The rule covers `GroupKFold`,
`StratifiedGroupKFold`, `GroupShuffleSplit`, `LeaveOneGroupOut` and `LeavePGroupsOut` in the
following places:

- as `cv` of `cross_val_score`, `cross_validate`, `cross_val_predict`, `learning_curve`,
  `validation_curve` and `permutation_test_score`
- in their own `split(X, y)`, called directly or through a name
- in a `GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV`, `HalvingRandomSearchCV` or
  `RFECV` whose `fit` gets no `groups=`

A call with `params=` or `**` is left alone, because it can carry the groups.
`SequentialFeatureSelector` and `LogisticRegressionCV` take groups only through metadata routing,
so the rule doesn't read them. Recall was 48 of 50.
