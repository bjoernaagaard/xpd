`OutlierRemover` drops rows of X and leaves y whole, so the model after it raises ValueError. Drop the outliers from X and y before the pipeline.

```diff lang="python"
-pipeline = make_pipeline(OutlierRemover(IsolationForest(), refit=True), LinearRegression())
-pipeline.fit(X, y)
+kept = IsolationForest().fit_predict(X) != -1
+model = LinearRegression().fit(X[kept], y[kept])
```

## What the rule reports

The rule reports `OutlierRemover` in a pipeline that also holds a step named like a regressor or a
classifier. The remover drops rows of X while the pipeline is fitted, and a scikit-learn pipeline
can't drop the same rows of y.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`fit` raised ValueError "Found input variables with inconsistent numbers of samples: [328, 400]".
A pipeline that ended in `KMeans`, which reads no y, fitted on the 328 kept rows, and the rule
leaves it alone.
