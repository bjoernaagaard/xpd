The skfolio `FactorModel` dataclass holds fitted results. Fit a `TimeSeriesFactorModel` prior with the factor matrix in the keyword-only `factors` argument.

```diff
-MeanRisk(prior_estimator=FactorModel())
-TimeSeriesFactorModel().fit(returns, factor_returns)
+MeanRisk(prior_estimator=TimeSeriesFactorModel())
+TimeSeriesFactorModel().fit(returns, factors=factor_returns)
```

## What the rule reports

To pass factors through an optimizer, request `factors` on the prior and pass the matrix to the optimizer's `fit`.
