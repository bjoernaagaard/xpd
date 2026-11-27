Refit preprocessing inside every cross-validation fold.

```diff lang="python"
-scaled = StandardScaler().fit_transform(X)
-scores = cross_val_score(model, scaled, y)
+scores = cross_val_score(make_pipeline(StandardScaler(), model), X, y)
```
