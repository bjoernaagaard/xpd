`predict` computes the same probabilities as `predict_proba` and returns the class with the largest one, so calling both does the work twice. Call `predict_proba` once and take the labels from it.

```diff lang="python"
 forest = RandomForestClassifier(random_state=0).fit(X_train, y_train)
-y_pred = forest.predict(X_test)
-y_score = forest.predict_proba(X_test)[:, 1]
+proba = forest.predict_proba(X_test)
+y_pred = forest.classes_[proba.argmax(axis=1)]
+y_score = proba[:, 1]
```

## What the rule reports

The rule reports `model.predict(X_test)` next to `model.predict_proba(X_test)` when the file builds
`model` as a `RandomForestClassifier`, `ExtraTreesClassifier` or `BaggingClassifier`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The rule covers the classes whose `predict` takes the argmax of `predict_proba`. Deriving the
labels from one `predict_proba` was 1.83 to 2.00 times faster for the three classes at 1,000 and
10,000 rows, with bit equal labels and probabilities. The rule leaves other classes
alone, because they decide the label from a raw score, where a tie at 0.5 can give another label. Neither reader called one
of the 22 held-back reports wrong. Recall was 30 of 30.
