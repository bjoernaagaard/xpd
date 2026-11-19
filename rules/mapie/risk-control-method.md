Each risk takes only some control methods, and some methods need a `confidence_level`, so `calibrate` raises here. Use a method that the risk takes, and pass `confidence_level`.

```diff lang="python"
-controller = MultiLabelClassificationController(model.predict_proba, risk="precision", method="crc")
+controller = MultiLabelClassificationController(model.predict_proba, risk="precision", confidence_level=0.9)
```

## What the rule reports

The rule reports `MultiLabelClassificationController` and `SemanticSegmentationController` with a
method that the risk doesn't take, or without a `confidence_level` that the method needs, or with
an argument that the method ignores. Each error comes from `calibrate`:

| Arguments | Result | Severity |
| --- | --- | --- |
| `risk="precision"` with `method="crc"` or `"rcps"` | "Invalid method 'crc' for risk 'precision'. Valid methods are: ['ltt']" | error |
| `method="ltt"` with recall | "Invalid method 'ltt' for risk 'recall'. Valid methods are: ['crc', 'rcps']" | error |
| `method="rcps"` or `risk="precision"`, with no `confidence_level` | "Invalid confidence_level. confidence_level cannot be `None` when using method 'rcps'", and the same for `'ltt'` | error |
| `method="crc"` beside a `confidence_level` or an `rcps_bound` | a UserWarning that the argument is ignored | warning |

## How to fix it

The severity is error for a method the risk does not take and for a missing `confidence_level`, and warning for an argument that `"crc"` ignores. xpd has no automatic fix. A missing `confidence_level` is a value that the file does not hold, and each other message names two rewrites. xpd has no automatic fix, so make the change by hand.
