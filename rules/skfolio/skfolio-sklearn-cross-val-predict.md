scikit-learn's `cross_val_predict` can reject skfolio financial splitters or convert portfolio predictions to a NumPy array.

```diff
-from sklearn.model_selection import cross_val_predict
-result = cross_val_predict(model, returns, cv=walk)
+from skfolio.model_selection import cross_val_predict
+result = cross_val_predict(model, returns, cv=walk)
```

## What the rule reports

skfolio returns a `MultiPeriodPortfolio` for WalkForward and a `Population` for combinatorial or randomized portfolio backtests. Check downstream code when changing the return type.
