scikit-learn GridSearchCV fits each skfolio estimator clone on each fold. skfolio provides online search for expanding evaluations that can reuse estimator state.

```diff
-GridSearchCV(MeanRisk(), grid, cv=WalkForward(10, 30, expand_train=True))
+OnlineGridSearch(MeanRisk(), grid, warmup_size=30, test_size=10)
```

## What the rule reports

Use online search when the prior and optimizer both support the required online updates. Match the schedule and scoring before comparing results.
