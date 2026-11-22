In linearmodels, `PanelOLS`, `PooledOLS`, `RandomEffects` and `FirstDifferenceOLS` default to unadjusted covariance. When errors within an entity are correlated over time, pass `cov_type="clustered", cluster_entity=True`.

```diff lang="python"
-result = PanelOLS(y, X, entity_effects=True).fit()
+result = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered", cluster_entity=True)
```

## What the rule reports

The rule reports `fit` with no `cov_type` on `PanelOLS`, `PooledOLS`, `RandomEffects` and
`FirstDifferenceOLS`. The default is `"unadjusted"`, the covariance for errors that are
homoskedastic and unrelated over time, but the errors of one entity are usually related.

## How to fix it

xpd has no automatic fix, so make the change by hand.

In two simulations with a true slope of 0, `PanelOLS` with entity effects gave p below 0.05 in
the following shares of the draws:

| Simulation | Default | `cov_type="robust"` | `cov_type="clustered"`, `cluster_entity=True` |
| --- | --- | --- | --- |
| 1,000 draws of 200 entities and 8 periods, with an AR(1) regressor and an AR(1) error within each entity, both with coefficient 0.8 | 17.3% | 16.4% | 5.7% |
| 300 entities and 3 periods, with errors that are heteroskedastic and unrelated over time | 22.4% | 7.1% | 5.0% |

How far the default is off depends on the data, so the rule is info. It is the rule with the most
reports, 372 in 142 of the 950 public files. `BetweenOLS` has one row for each entity and
`FamaMacBeth` has its own covariance, so both are left alone. A call that states `cov_type`, also
as `"unadjusted"`, is left alone, because the choice is written out.
