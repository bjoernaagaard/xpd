A regularization or smoothing of 0 codes a category that holds one class alone as inf, without an error. Pass a value above 0.

```diff lang="python"
-encoder = ce.WOEEncoder(cols=["city"], regularization=0)
+encoder = ce.WOEEncoder(cols=["city"], regularization=0.5)
```

## What the rule reports

The rule reports `WOEEncoder(regularization=0)` and `CountTargetEncoder(smoothing=0)`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`WOEEncoder` adds `regularization`, 1.0 by default, to the counts of both classes before it takes
the logarithm of their ratio. With 0, a category that holds one class alone divides by zero.

| Form | Result |
| --- | --- |
| `WOEEncoder(regularization=0)` on a category of five rows of class 1 | `inf`, with no error and no warning |
| `WOEEncoder()` on the same category | 1.204 |
| `WOEEncoder(regularization=0)` on a column where every category holds both classes | finite codes |
| `CountTargetEncoder(smoothing=0)` | `ValueError: smoothing must be strictly bigger than 0.` at fit |
| `TargetEncoder(smoothing=0)` or `MEstimateEncoder(m=0)`, left alone | the plain mean of each category |

The `CountTargetEncoder` document carries `since: "2.11"`, the release that added the class.
