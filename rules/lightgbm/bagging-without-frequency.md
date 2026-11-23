LightGBM leaves row bagging disabled at its default frequency of zero.
Setting a row fraction between zero and one has no effect on ordinary
bagging unless the frequency is positive.

```diff
-model = lgb.LGBMRegressor(subsample=0.8)
+model = lgb.LGBMRegressor(subsample=0.8, subsample_freq=1)
```

## How to fix it

Set `subsample_freq` on a scikit-learn estimator or `bagging_freq` in
native `train()` or `cv()` parameters. A frequency of one samples rows
on every boosting round. If you meant to use all rows, remove the fraction.

The rule checks simple decimal fractions and a zero or omitted frequency.
It skips competing aliases, explicit sampling or boosting strategies and
settings unpacked from another object.

[Bagging parameters](https://lightgbm.readthedocs.io/en/v4.7.0/Parameters.html#bagging_freq)
