Picking a setting by its score on the test rows makes the best score too high. Pick it with cross-validation on the training rows, and score the test rows once.

```diff lang="python"
-for k in range(1, 30):
-    model = KNeighborsClassifier(n_neighbors=k).fit(X_train, y_train)
-    scores.append(model.score(X_test, y_test))
-best = max(scores)
+search = GridSearchCV(KNeighborsClassifier(), {"n_neighbors": range(1, 30)}, cv=5)
+search.fit(X_train, y_train)
+test_score = search.score(X_test, y_test)
```

## What the rule reports

The rule reports a loop that builds one model per setting, scores each model on the test rows and
keeps the best score.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The best of 30 test accuracies for `n_neighbors` from 1 to 30 was 0.597, where a `GridSearchCV`
on the training rows gave 0.507. The rule needs all of the following in the loop:

1. The loop variable is a keyword of the call that built the scored model.
2. A `score`, `predict` or `predict_proba` runs on a name with `test` in it.
3. The loop picks a setting. A pick is a `>` comparison that reads or sets a name with `best` or
   `max` in it, or whose branch reads the setting. A later `max`, `argmax` or `idxmax` of the
   list that the loop appends to is a pick too.

A keyword named like `random_state`, `seed`, `method` or `model` is left alone, because such a
loop compares or averages.

The rule as the readers saw it gave wrong advice in 2 and 2 of 3 labelled reports, on loops that
printed whether the training score was far above the test score. The narrowed rule no longer
makes the reports. Recall was 15 of 50. The rule found none of 20 thresholds picked by the test
score. It also misses scores kept in a dict and picked with `max(results, key=results.get)`, a
loop over `ParameterGrid` that builds the model with `**params`, and a list comprehension picked
with `np.argmax`.
