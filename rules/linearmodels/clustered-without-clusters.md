A clustered fit in linearmodels needs the intended groups. With no groups, models such as `PanelOLS` and `IV2SLS` put each observation in its own cluster, while `IVGMM` raises an error. Pass `clusters=` with the group labels, or use `cluster_entity=True` or `cluster_time=True` on a panel model that supports it.

```diff lang="python"
-result = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered")
+result = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered", cluster_entity=True)
```

## What the rule reports

The rule reports `fit(cov_type="clustered")` with none of `cluster_entity`, `cluster_time` and
`clusters`. Models that accept omitted groups put each observation in its own cluster,
which gives the heteroskedasticity robust covariance. `IVGMM`, `IVGMMCUE` and `IVJIVE` require groups.

## How to fix it

xpd has no automatic fix. Because the code does not show whether to cluster by entity or by time, and the groups for `clusters=` are not in the file. Make the change by hand.

On 7.1 and 7.0, `PanelOLS` with entity effects on the wage panel gave the following first standard errors:

| `fit` | First standard error |
| --- | --- |
| `cov_type="clustered"` | 0.041301 |
| `cov_type="robust"` | 0.041301 |
| `cov_type="clustered", cluster_entity=True` | 0.049385 |

`PooledOLS`, `RandomEffects` and `FirstDifferenceOLS` gave equal pairs in the same way. So did
`OLS`, `IV2SLS`, `IVLIML`, `AbsorbingLS`, `SUR` and `IV3SLS`, which take the groups as `clusters=`,
such as 0.427716 both ways on `IV2SLS`. `IVGMM`, `IVGMMCUE` and `IVJIVE` raise `TypeError` instead,
and the rule's message for them says so.

The rule reports 31 calls in 9 of the 950 public files, each a regression table whose standard errors
are said to be clustered and are not. `BetweenOLS` is left out, because it refuses `cluster_entity`
and its rows are the entities already.
