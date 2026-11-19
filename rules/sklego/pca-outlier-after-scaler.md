After a centring step such as `StandardScaler`, the default relative error of `PCAOutlierDetection` divides by numbers near zero and flags many normal rows. Pass `variant="absolute"`.

```diff lang="python"
-detector = make_pipeline(StandardScaler(), PCAOutlierDetection(n_components=1, threshold=0.05))
+detector = make_pipeline(StandardScaler(), PCAOutlierDetection(n_components=1, threshold=2.0, variant="absolute"))
```

## What the rule reports

The rule reports `PCAOutlierDetection` and `UMAPOutlierDetection` without `variant="absolute"` in a
pipeline that holds `StandardScaler`, `RobustScaler` or `PowerTransformer`. The default
`variant="relative"` divides the reconstruction error of a row by the sum of the row's values,
which a centring step puts near zero.

## How to fix it

xpd has no automatic fix, so make the change by hand.

xpd gives the replacement as advice and doesn't apply it, because the threshold has other
units.

| Data, 1,000 rows near a line with 20 planted outliers | Rows flagged by the relative variant, with `threshold=0.1` | By `variant="absolute"`, with `threshold=2.0` |
| --- | --- | --- |
| raw | the 20 planted rows | the 20 planted rows |
| after `StandardScaler` | 320, of which 19 were planted | the 20 planted rows |

After the scaler, 510 rows had a negative ratio. A scaler built with `with_mean=False`,
`with_centering=False` or `standardize=False` doesn't centre, and the rule leaves it alone.
