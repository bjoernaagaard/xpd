A lambda inside named aggregation runs Python once for every group. Compute its expression as a column before the groupby, and name a built-in reduction in the pair.

```diff lang="python"
-found = table.groupby("area").agg(
-    share=("gap", lambda values: (values <= CENT).mean()),
-    rmse=("error", lambda error: np.sqrt((error**2).mean())),
-)
+table = table.assign(near=table["gap"] <= CENT, squared=table["error"] ** 2)
+found = table.groupby("area").agg(share=("near", "mean"), mse=("squared", "mean"))
+found = found.assign(rmse=np.sqrt(found["mse"]))
```

## What the rule reports

The rule reports a lambda inside a pair of named aggregation, as in
`agg(share=("gap", lambda v: (v <= CENT).mean()))`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The lambda runs in Python once per group. The condition and the square are the same for every
row, so they are columns, and the reduction that is left has a built-in name that pandas runs
on all groups at once. `groupby-lambda` reports a lambda passed to `agg` directly.
