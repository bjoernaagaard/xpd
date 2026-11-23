LightGBM 4.7 deprecates `eval_set` on estimator `fit()`. Passing a non-None
`eval_set` together with a non-None `eval_X` or `eval_y` also raises
`ValueError`.

```diff
-model.fit(X_train, y_train, eval_set=[(X_val, y_val)])
+model.fit(X_train, y_train, eval_X=(X_val,), eval_y=(y_val,))
```

## How to fix it

Pass both `eval_X` and `eval_y`. For multiple validation sets, pass a tuple
of feature matrices and a tuple of label arrays in the same order.
For a single set, you can also pass the matrix and labels directly.

On LightGBM 4.7 or later, use `xpd check --fix --unsafe-fixes` to rewrite
`eval_set=[(X_val, y_val)]` when both values are simple names and the other
arguments are explicit. The fix preserves the validation data. It requires
the new API, so confirm the target version when xpd cannot find an installed
version. Migrate positional arguments, multiple sets, and calls already
passing replacement keywords by hand.

[Estimator source](https://lightgbm.readthedocs.io/en/v4.7.0/_modules/lightgbm/sklearn.html)
