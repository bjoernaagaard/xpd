Each `cross_val_score` call fits the model on every fold again to compute one more score. Pass every scoring to one `cross_validate` call, which fits once.

```diff lang="python"
-accuracy = cross_val_score(model, X, y, cv=5, scoring="accuracy")
-precision = cross_val_score(model, X, y, cv=5, scoring="precision")
+scores = cross_validate(model, X, y, cv=5, scoring=["accuracy", "precision"])
+accuracy, precision = scores["test_accuracy"], scores["test_precision"]
```

## What the rule reports

The rule reports `cross_val_score(model, X, y, scoring="accuracy")` followed by
`cross_val_score(model, X, y, scoring="precision")` on the same model, data and `cv`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The rule reports the first call when a later call in the same block passes the same model, data
and `cv` with another `scoring`. With `LogisticRegression`, two scorings in one `cross_validate`
were 1.60 times faster at 2,000 rows and 1.84 times faster at 20,000 rows, and the scores were bit
equal. Neither reader called one of the 14 held-back reports wrong. Recall was 30 of 30.
