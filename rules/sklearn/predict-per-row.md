Calling `predict` once per row checks the input and sets up the model again for every row. Call it once on all the rows, which was at least 2.7 times faster.

```diff lang="python"
-labels = [model.predict([row])[0] for row in X_test]
+labels = model.predict(X_test)
```

## What the rule reports

The rule reports `model.predict([row])`, `model.predict_proba(X[i].reshape(1, -1))`,
`model.predict(df.iloc[[i]])` and the same calls of `decision_function` and `predict_log_proba` in
a loop, a comprehension or a lambda over the rows.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The rule reports a call whose rows come from the loop variable, either the row itself or the data
indexed by it. Each call checks the input and sets up the model again, so one call on all rows was
faster:

| Model | At 500 rows | At 5,000 rows |
| --- | --- | --- |
| `SVC` | 2.77 times | 2.79 times |
| `RandomForestClassifier` | 216 times | 356 times |
| `LogisticRegression.predict_proba` | 104 times | 816 times |

The labels were bit equal. The probabilities of `LogisticRegression` differed by up to 3.3e-16,
so the rule has no fix. A loop that writes into the data it predicts from, as recursive
forecasting does, is left alone.

Reader B called both held-back reports wrong advice, and reader A called neither wrong. Recall was
28 of 32. The 4 misses were `DataFrame.apply` with the row's values in a list, as in
`X_test.apply(lambda row: model.predict([row.values])[0], axis=1)`.
