A skfolio convex optimizer can fail on one fold when the constraints or estimated inputs change.

```diff
-cross_val_predict(MeanRisk(), returns, cv=walk)
+cross_val_predict(MeanRisk(fallback=EqualWeighted()), returns, cv=walk)
```

## What the rule reports

Only use a fallback that satisfies your portfolio requirements. If you opt into failed portfolios, account for missing predictions in evaluation.
