`KFold` uses `random_state` only when it shuffles, so without `shuffle=True` the call raises ValueError. Pass `shuffle=True`, or drop `random_state`.

```diff lang="python"
-folds = KFold(n_splits=5, random_state=42)
+folds = KFold(n_splits=5, shuffle=True, random_state=42)
```

## What the rule reports

The rule reports `KFold(n_splits=5, random_state=42)` without `shuffle=True`, which raises
ValueError.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The rule covers `KFold`, `StratifiedKFold` and `StratifiedGroupKFold`, and `GroupKFold` from
scikit-learn 1.6, which added its `shuffle`. Each raises "Setting a random_state has no effect
since shuffle is False" on scikit-learn 1.9.1. A file that imports the removed
`sklearn.cross_validation` is left alone unless the call passes `n_splits`, because the old classes
took other arguments. `removed-in-sklearn` reports the import in such a file.

The rule as the readers saw it gave wrong advice in 2 and 2 of 11 labelled reports, on such files,
and the narrowed rule no longer makes the reports. Recall was 54 of 54.
