skfolio HRP and Schur configure their clustering through a `HierarchicalSeriation` estimator.

```diff
-HierarchicalRiskParity(hierarchical_clustering_estimator=cluster)
+HierarchicalRiskParity(seriation_estimator=HierarchicalSeriation(hierarchical_clustering_estimator=cluster))
```

## What the rule reports

The rule covers the two constructors and the legacy fitted alias. HERC still accepts `hierarchical_clustering_estimator`. NCO accepts `clustering_estimator`.
