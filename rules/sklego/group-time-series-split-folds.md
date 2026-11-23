`GroupTimeSeriesSplit` tries every combination of split points, so each extra fold multiplies the time, from about 1 second at 5 folds to 10 at 6. Use 5 folds or fewer, or merge groups into coarser periods first.

```diff lang="python"
-folds = GroupTimeSeriesSplit(n_splits=6)
+folds = GroupTimeSeriesSplit(n_splits=5)
```

## What the rule reports

The rule reports `GroupTimeSeriesSplit` with 6 or more splits. The class tries every combination of
split points over the groups to find the most even folds, so each extra fold multiplies the time.

## How to fix it

With `--unsafe-fixes`, xpd sets a literal fold count above five to five.

| Data | `split` at 4 splits | At 5 splits | At 6 splits |
| --- | --- | --- | --- |
| 3,000 rows in 60 groups | 0.09 s | 1.08 s | 10.48 s |
| 3,000 rows in 40 groups, with the default number of threads | 0.015 s | 0.10 s | 1.07 s |

The class warns itself above 5 splits when there are more than 30 groups. With a few groups the search is short, so
the rule is a warning.
