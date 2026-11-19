`EstimatorTransformer` reshapes its output to one column, so `predict_proba` comes back with twice as many rows and a pipeline around it raises. Use `predict_func="decision_function"` for a binary model, or scikit-learn's `StackingClassifier`.

```diff lang="python"
-features = EstimatorTransformer(LogisticRegression(), predict_func="predict_proba")
+features = EstimatorTransformer(LogisticRegression(), predict_func="decision_function")
+stack = StackingClassifier([("lr", LogisticRegression())], stack_method="predict_proba")
```

## What the rule reports

The rule reports `EstimatorTransformer(..., predict_func="predict_proba")` and
`"predict_log_proba"`. When the target has one dimension, the transformer reshapes the output to
one column, so the probabilities of the two classes stack into twice as many rows.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A binary model on 400 rows came back from `transform` with the shape (800, 1). Inside a
`FeatureUnion`, and inside a pipeline before a model, `fit` raised ValueError on the row counts.
`decision_function` gave (400, 1), and `StackingClassifier(stack_method="predict_proba").transform`
gave one row per input row.
