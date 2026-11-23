LightGBM `train()` and `cv()` use `num_iterations` or an alias in
`params` ahead of the `num_boost_round` argument. This call uses 50 rounds
as its limit.

```diff
-lgb.train({"n_estimators": 50}, train_set, num_boost_round=200)
+lgb.train({}, train_set, num_boost_round=50)
```

## How to fix it

Check which value you intended and keep it in one place. The rule reports
different positive decimal integers when the `params` dictionary has
exactly one iteration key and the call passes `num_boost_round` by keyword.

[Training parameters](https://lightgbm.readthedocs.io/en/v4.7.0/Parameters.html#num_iterations) |
[Training engine](https://lightgbm.readthedocs.io/en/v4.7.0/_modules/lightgbm/engine.html)
