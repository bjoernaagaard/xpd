A `FunctionTransformer` that computes statistics of its input scales test rows by test statistics, so a prediction changes with the other rows in the batch. Use `StandardScaler` or `MinMaxScaler`, which learn the statistics at fit.

```diff lang="python"
-pipe = make_pipeline(FunctionTransformer(lambda X: (X - X.mean(axis=0)) / X.std(axis=0)), model)
+pipe = make_pipeline(StandardScaler(), model)
```

## What the rule reports

The rule reports `FunctionTransformer(lambda X: (X - X.mean(axis=0)) / X.std(axis=0))`, a function
that takes statistics of the rows it gets. At predict time it then scales by the test rows.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The rule is info, because on data with no signal it doesn't move a score. Predicted in batches of
5 rows it scored 0.484 against 0.494 for `StandardScaler`. On data with signal, over 30 seeds,
`LogisticRegression` scored lower behind the function:

| Batch of test rows | Behind the function | Behind `StandardScaler` |
| --- | --- | --- |
| 5 | 0.773 | 0.847 |
| 20 | 0.824 | 0.847 |

A reduction along axis 1 works per row and is left alone. Recall was 24 of 30, and the rule found
none of 6 functions defined with `def` and passed by name.
