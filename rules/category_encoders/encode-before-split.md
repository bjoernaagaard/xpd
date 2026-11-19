A `TargetEncoder` fitted on every row before the split puts each test row's own label into its code, which scored 0.655 AUC on data with no signal. Split first, then fit the encoder on the training part.

```diff lang="python"
 from category_encoders import TargetEncoder
 
-X_encoded = TargetEncoder(cols=["city"]).fit_transform(X, y)
-X_train, X_test, y_train, y_test = train_test_split(X_encoded, y, random_state=0)
+X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
+encoder = TargetEncoder(cols=["city"])
+X_train = encoder.fit_transform(X_train, y_train)
+X_test = encoder.transform(X_test)
```

## What the rule reports

The rule reports `TargetEncoder(...).fit_transform(X, y)` on every row, when the file imports
`TargetEncoder` from `category_encoders` and the result is later split by `train_test_split` or
passed to `cross_val_score` or another cross-validation function.

## How to fix it

xpd has no automatic fix, so make the change by hand.

| Old form, on data with no signal | Test AUC | Right form |
| --- | --- | --- |
| `TargetEncoder` fitted on every row before the split | 0.655, 19.8 standard errors above 0.5 | 0.495 with the split first |
| `TargetEncoder` fitted on every row before `cross_val_score` | 0.650 | 0.502 for a Pipeline that holds the encoder and the model |
| `LeaveOneOutEncoder`, `WOEEncoder`, `JamesSteinEncoder`, `MEstimateEncoder` or `CountTargetEncoder` fitted on every row before the split, and `CatBoostEncoder` fitted with `fit(X, y)` and then `transform(X)` | 0.654 to 0.688 | not given |
| `CatBoostEncoder(...).fit_transform(X, y)` on every row before the split | 0.490, because it codes each row from the labels of the rows before it | not given |

[`fit-before-split`](../sklearn/fit-before-split.md) and
[`cross-val-on-fitted-data`](../sklearn/cross-val-on-fitted-data.md) of the scikit-learn pack report the form
for every step but one, a bare `TargetEncoder(...)` built in the same expression. They take it for
the scikit-learn class, which cross-fits in `fit_transform` and so never codes a row from its own
label. The
category_encoders class doesn't, so `encode-before-split` reports the form when the import shows
which class it is. The scikit-learn pack reports every other form, such as `ce.TargetEncoder(...)`,
an encoder kept in a name, or another encoder class.
