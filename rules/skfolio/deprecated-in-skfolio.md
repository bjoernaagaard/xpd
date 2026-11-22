skfolio plans to remove its deprecated hierarchical import paths, `annualized_factor` and `non_denominated_sort` in 2.0.

```diff
-from skfolio.optimization.cluster import HierarchicalRiskParity
-Portfolio(returns, weights, annualized_factor=252)
-population.non_denominated_sort()
+from skfolio.optimization.hierarchical import HierarchicalRiskParity
+Portfolio(returns, weights, annualization_factor=252)
+population.non_dominated_sort()
```

## What the rule reports

The keyword fix keeps the annualization value and type. On ImpliedCovariance, the old name is also an estimator parameter. Code that inspects the constructor or relies on warning capture needs separate review.
