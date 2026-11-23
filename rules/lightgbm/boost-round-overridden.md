LightGBM `train()` and `cv()` use `num_iterations` or an alias in
`params` ahead of the `num_boost_round` argument. This call uses 50 rounds
as its limit.

```diff
-lgb.train({"n_estimators": 50}, train_set, num_boost_round=200)
+lgb.train({"n_estimators": 50}, train_set)
```

## How to fix it

The safe fix removes the ignored keyword argument or a final positional
round count. It keeps the effective value in `params`. With later
positional arguments, move them to keywords before dropping the round
count. To change the number of rounds, set the intended value in `params`
by hand.

The rule checks an explicit positive integer against a single iteration
key or the primary `num_iterations` key. In current LightGBM the primary
key wins even when a dictionary alias is also present.

[Training parameters](https://lightgbm.readthedocs.io/en/v4.7.0/Parameters.html#num_iterations) |
[Training engine](https://lightgbm.readthedocs.io/en/v4.7.0/_modules/lightgbm/engine.html)
