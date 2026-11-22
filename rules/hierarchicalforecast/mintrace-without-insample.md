`MinTrace(method="mint_shrink")` estimates forecast error covariance from in-sample residuals. Give `reconcile` the historical targets and the fitted predictions of the base models.

```diff lang="python"
  reconciliation = HierarchicalReconciliation(reconcilers=[MinTrace(method="mint_shrink")])
- forecasts = reconciliation.reconcile(Y_hat_df=base, S_df=summing, tags=tags)
+ forecasts = reconciliation.reconcile(Y_hat_df=base, S_df=summing, tags=tags, Y_df=fitted)
```

## What the rule reports

The rule looks for a literal `MinTrace("mint_shrink")` in the list or tuple of reconcilers, either in a constructor chained to `reconcile` or assigned on the previous statement. It reports a missing `Y_df` or an explicit `Y_df=None` when no positional argument or unpacked options could supply it. Temporal reconciliation has different requirements and is outside this check.

HierarchicalForecast 1.5.3 raised `ValueError` asking for `Y_df`. With fitted values from StatsForecast, the reconciled total equaled the sum of two leaves on all seven forecast dates. See the [reconciliation API and worked example](https://nixtlaverse.nixtla.io/hierarchicalforecast/core.html).

## How to fix it

Fit the base models on the training set and pass a frame with one row per series and training time, the actual target, and a fitted column for each base model. For StatsForecast, call `forecast(..., fitted=True)` and then `forecast_fitted_values()`. xpd has no automatic fix.
