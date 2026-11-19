A cluster or kernel option in a `fit` call without the matching `cov_type` raises an error instead of clustering. Add `cov_type="clustered"` or the kernel covariance it belongs to.

```diff lang="python"
-result = PanelOLS(y, X, entity_effects=True).fit(cluster_entity=True)
-fama = FamaMacBeth(y, X).fit(bandwidth=3)
+result = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered", cluster_entity=True)
+fama = FamaMacBeth(y, X).fit(cov_type="kernel", bandwidth=3)
```

## What the rule reports

The rule reports `cluster_entity`, `cluster_time` or `clusters` in a `fit` call whose `cov_type` is
absent or is a name other than `"clustered"`. It also reports `bandwidth` or `kernel` in a call
whose `cov_type` is absent or is not a kernel covariance.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix adds `cov_type="clustered"` or `cov_type="kernel"` to a call that names no `cov_type`. The call raised before the fix, except on `FamaMacBeth` and on the factor models of 7.0, where the fix changes the standard errors on purpose. A call that names another `cov_type` has no fix, because the code doesn't show which covariance is meant.

7.1 raised `ValueError: The keyword argument 'cluster_entity' is not accepted with
cov_type='unadjusted'` on `PanelOLS`. Each of the 15 classes other than `FamaMacBeth` and the
factor models was run with each of the five keywords, against no `cov_type` and against each name
the rule reads, and every call raised:

| Release | Calls | Errors |
| --- | --- | --- |
| 7.1 | 930 | `ValueError` or `UnknownOptionError` |
| 7.0 | 868 | `ValueError`, `TypeError` or `KeyError` |

`FamaMacBeth.fit` names `bandwidth` and `kernel` in its signature and reads them only with
`cov_type="kernel"`. On 7.1 and 7.0 the first standard error on the wage panel was:

| `FamaMacBeth(y, X).fit(...)` | First standard error |
| --- | --- |
| no option | 0.203664 |
| `bandwidth=3` | 0.203664 |
| `kernel="parzen"` | 0.203664 |
| `cov_type="kernel", bandwidth=3` | 0.300158 |

The factor models ignored the two keywords on 7.0, where `TradedFactorModel.fit(bandwidth=3)`
returned the standard error of the default, and they raise `ValueError` on 7.1.
