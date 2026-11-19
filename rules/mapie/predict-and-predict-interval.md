`predict_interval` already returns the point predictions as its first value, so calling `predict` too runs the model twice. Unpack both from one `predict_interval` call.

```diff lang="python"
-y_pred = mapie.predict(X_test)
-_, y_pis = mapie.predict_interval(X_test)
+y_pred, y_pis = mapie.predict_interval(X_test)
```

## What the rule reports

The rule reports `predict` on the same object and the same rows as a `predict_interval` or
`predict_set` in the same block.

## How to fix it

xpd has no automatic fix. The rewrite needs a new name and changes two statements. Make the change by hand.

`predict_interval` and `predict_set` return the point predictions as their first value. With a
forest of 50 trees on one thread, both forms returned the same arrays, bit for bit, and the pair
of calls took longer than the single call:

| Calls | At 2,000 rows | At 20,000 rows |
| --- | --- | --- |
| `predict` and `predict_interval`, against `predict_interval` alone | 1.99 times | 1.99 times |
| `predict` and `predict_set`, against `predict_set` alone | 1.98 times | 1.97 times |

The two calls must be statements of the same block and name the same object and the same rows.
