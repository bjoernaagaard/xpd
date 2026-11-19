`model.score(X, y)` predicts `X` again when you already hold the predictions. Compute the metric from the predictions you have.

```diff lang="python"
 forest = RandomForestClassifier(random_state=0).fit(X_train, y_train)
 y_pred = forest.predict(X_test)
-accuracy = forest.score(X_test, y_test)
+accuracy = accuracy_score(y_test, y_pred)
```

## What the rule reports

The rule reports `model.score(X_test, y_test)` and `f1_score(y_test, model.predict(X_test))` after
`y_pred = model.predict(X_test)`. It reports them only when the file builds `model` as a forest, a
boosting or bagging model, k-nearest neighbours, an SVM, an MLP or `GaussianNB`.

The rule leaves both decision trees alone, as it does `LogisticRegression` and `LinearRegression`.
A name that the file also binds to another class is left alone.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `f1_score(y_test, y_pred)`, and `accuracy_score(y_test, y_pred)` or `r2_score(y_test, y_pred)` for `model.score(X_test, y_test)` when the file binds the name to a classifier or to a regressor. Code between the two lines can change the predictions. A name bound to both has no fix.

The rule reads the class from the line that builds the model, because `score` costs the most
where `predict` does. Over the 13 reported classes that were timed, at 1,000 and 10,000 test rows,
the score call took 1.62 to 879 times as long as the metric alone, and gave the same number.
`GaussianNB` had the smallest gain, 1.62 times at 1,000 rows. The decision trees fell below 1.5
times at 1,000 rows in one of two runs:

| Class | First run | Saved rerun |
| --- | --- | --- |
| `DecisionTreeClassifier` | 1.45 times | 1.54 times |
| `DecisionTreeRegressor` | 1.52 times | 1.66 times |

The fix on the metric form is unsafe, because code between the two lines can change the saved
predictions. It gave the same score in 12 of 12 runs on both held-back shapes, and the score
changed only when the model was fitted again between the two lines. Neither reader called one of
the 17 held-back reports wrong. Recall was 31 of 32.
