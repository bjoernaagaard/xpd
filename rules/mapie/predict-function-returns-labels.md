`BinaryClassificationController` needs a function that returns class probabilities, and `predict` returns labels, so `calibrate` raises IndexError. Pass `model.predict_proba`.

```diff lang="python"
-controller = BinaryClassificationController(model.predict, "precision", target_level=0.8)
+controller = BinaryClassificationController(model.predict_proba, "precision", target_level=0.8)
```

## What the rule reports

The rule reports `BinaryClassificationController` with `model.predict` as its `predict_function`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `model.predict_proba`.

With `model.predict`, `calibrate` raised IndexError, "Error when calling the predict_function",
because the controller reads the probability of the positive class from the second column.
`MultiLabelClassificationController` ran with `predict`, so the rule leaves it alone.
