skfolio defaults `purged_size` and `embargo_size` to zero. Overlapping observations or labels can cross a split boundary without a gap.

```diff
-CombinatorialPurgedCV(n_folds=5, n_test_folds=2)
+CombinatorialPurgedCV(n_folds=5, n_test_folds=2, purged_size=1, embargo_size=1)
```

## What the rule reports

In a 60-row run the first training split had 36 rows with no gap and 34 with both gaps set to one. Select the gap sizes for the overlap in your own data.
