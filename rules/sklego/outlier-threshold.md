These outlier detectors need a valid `threshold`, and fit raises ValueError without one or with one out of range. Pass `threshold=` with a value the detector takes.

```diff lang="python"
-pca = PCAOutlierDetection(n_components=2)
-gmm = GMMOutlierDetector(threshold=0.05)
+pca = PCAOutlierDetection(n_components=2, threshold=0.1)
+gmm = GMMOutlierDetector(threshold=0.95)
```

## What the rule reports

The rule reports `PCAOutlierDetection` and `UMAPOutlierDetection` built without a `threshold`. It
also reports `GMMOutlierDetector` and `BayesianGMMOutlierDetector` whose `threshold` is below 0.5
or above 1, with `method="quantile"` written out or left as the default.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Without a threshold, `fit` of the PCA and UMAP detectors raised ValueError "The `threshold` value
cannot be `None`." The mixture detectors keep the share `threshold` of the training rows as
inliers:

| `threshold` | Training rows flagged, of 400 |
| --- | --- |
| 0.05 | 380 |
| 0.4 | 240 |
| 0.5 | 200 |
| 0.95 | 20 |
| 0.99, the default | 4 |
| 2 | ValueError |

`threshold=2` ran with `method="stddev"`, which counts standard deviations. A call whose `method`
is anything but the text `"quantile"`, such as a variable, is left alone, because it may be
`"stddev"`. A detector inside a search is left alone, because the search may set the threshold.
