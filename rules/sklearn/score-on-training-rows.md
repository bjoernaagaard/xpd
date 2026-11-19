Scoring a model on the rows it was fitted on says nothing about new data, and a decision tree scored 1.00 this way on data with no signal. Score it on test rows the fit never saw.

```diff lang="python"
 model.fit(X_train, y_train)
-print(model.score(X_train, y_train))
+print(model.score(X_test, y_test))
```

## What the rule reports

The rule reports `model.score(X_train, y_train)` and
`accuracy_score(y_train, model.predict(X_train))` when nothing around them scores held-out rows.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The rule reads the names of the rows. On noise, each model scored higher on its training rows:

| Model | Accuracy on the training rows | On the test rows |
| --- | --- | --- |
| `DecisionTreeClassifier` with default settings | 1.000 | 0.510 |
| 1-nearest-neighbour | 1.000 | 0.495 |
| `LogisticRegression` | 0.545 | 0.501 |

The rule stays silent when code nearby scores a name with `test`, `val` or `holdout` in it, or
calls cross-validation or a search. Nearby means the function around the score, the functions next
to it, or the file for code outside functions.

Recall was 10 of 27. The rule found none of 9 metrics given a `predict` variable of the training
rows, as in `y_pred = knn.predict(X_train)` and `accuracy_score(y_train, y_pred)`. It also misses
rows whose names don't say train, as in `model.score(X, y)` after `model.fit(X, y)`.
