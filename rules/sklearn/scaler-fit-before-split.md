Fit scalers and imputers on training rows only.

```diff lang="python"
-scaled = StandardScaler().fit_transform(X)
-X_train, X_test = train_test_split(scaled)
+X_train, X_test = train_test_split(X)
+scaler = StandardScaler().fit(X_train)
```
