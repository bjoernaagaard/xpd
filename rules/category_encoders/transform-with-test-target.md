Passing the test labels to `transform` makes a supervised encoder use each test row's own label, which leaks the answer into the features. Call `transform(X)` without the target.

```diff lang="python"
 encoder = ce.LeaveOneOutEncoder(cols=["city"])
 X_train = encoder.fit_transform(X_train, y_train)
-X_test = encoder.transform(X_test, y_test)
+X_test = encoder.transform(X_test)
```

## What the rule reports

The rule reports `encoder.transform(X_test, y_test)`. The supervised encoders of category_encoders
take the target in `transform`, which no scikit-learn transformer does, and the target there is
meant for the training rows.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is error when the encoder is a `LeaveOneOutEncoder`, and info for the other supervised encoders. The unsafe fix writes `encoder.transform(X_test)`.

| Encoder | Test AUC with `transform(X_test, y_test)` | With `transform(X_test)` |
| --- | --- | --- |
| `LeaveOneOutEncoder` | 0.691, 11.4 standard errors above 0.5 | 0.517 |
| `CatBoostEncoder` | 0.499, 0.1 standard errors from 0.5 | 0.516 |
| `TargetEncoder`, `WOEEncoder`, `JamesSteinEncoder`, `MEstimateEncoder`, `QuantileEncoder`, `CountTargetEncoder` | the same as without the test labels | the same |

With the target, `LeaveOneOutEncoder` takes each row's own label out of the mean of its category,
so the code of a test row moves with its label. The other supervised encoders read the target in
`transform` only to add the noise of `sigma` or `randomized=True`. The form is still not meant for
test rows, so it is info for them.

The rule reads the name of the first argument, which must hold `test` or `holdout`, or be a
validation name such as `X_val`. It needs the encoder assigned from a supervised class in the same
function, or in the module for code outside a function.

The fix is unsafe. It changes the codes of the test rows of a `LeaveOneOutEncoder` or a
`CatBoostEncoder`, and it takes the noise off the test rows of an encoder with `randomized=True`.
On a `TargetEncoder` and a `WOEEncoder` without noise, the fixed line gave an equal frame.
