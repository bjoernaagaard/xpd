LightGBM 4.7 deprecates `eval_set` on estimator `fit()`. Passing a non-None
`eval_set` together with either new validation argument also raises
`ValueError`.

```diff
-model.fit(X_train, y_train, eval_set=[(X_val, y_val)])
+model.fit(X_train, y_train, eval_X=(X_val,), eval_y=(y_val,))
```

## How to fix it

Pass both `eval_X` and `eval_y`. For multiple validation sets, pass a tuple
of feature matrices and a tuple of label arrays in the same order.
For a single set, you can also pass the matrix and labels directly.

Migrate by hand after updating LightGBM. xpd can report current API advice
when the installed version is unknown.

[Estimator source](https://lightgbm.readthedocs.io/en/v4.7.0/_modules/lightgbm/sklearn.html)
