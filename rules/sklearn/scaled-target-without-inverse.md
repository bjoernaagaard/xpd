Restore predictions to their original units before scoring a transformed target.

```diff lang="python"
-log_target = np.log1p(y)
+regressor = TransformedTargetRegressor(regressor=model, func=np.log1p, inverse_func=np.expm1)
```
