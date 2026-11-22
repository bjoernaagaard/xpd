Repeatedly calling fit resets a skfolio estimator's state. An online evaluator can reuse state through partial_fit on an expanding schedule.

```diff
-for train, test in WalkForward(10, 30, expand_train=True).split(returns):
-    model.fit(returns.iloc[train])
+result = online_predict(model, returns, warmup_size=30, test_size=10)
```

## What the rule reports

The rule covers direct expanding WalkForward loops. Check prior support and the rebalance schedule before migrating.
