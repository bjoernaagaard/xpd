Fitting on the test rows makes a score on those rows meaningless, and a model fitted this way scored 0.997 AUC on data with no signal. Fit on the training rows only.

```diff lang="python"
-model = LogisticRegression().fit(X_test, y_test)
-X_test = scaler.fit_transform(X_test)
+model = LogisticRegression().fit(X_train, y_train)
+X_test = scaler.transform(X_test)
```

## What the rule reports

The rule reports `model.fit(X_test, y_test)` and `model.partial_fit(X_test, y_test)`, which train
on the test rows. It also reports `scaler.fit_transform(X_test)` and `scaler.fit(X_test)`, which
refit a step on them.

The rule leaves alone `PermutationImportance` and `permutation_importance`, a fit on train and
test rows joined, a `LabelBinarizer` or `MultiLabelBinarizer`, and a `LabelEncoder` whose result
goes straight into another call, such as a plot.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is error for `fit` with a target and for `partial_fit`, and info for `fit_transform` and for `fit` without a target. The unsafe fix writes `scaler.transform(X_test)` for `scaler.fit_transform(X_test)`, and `selector.transform(X_test)` for `selector.fit_transform(X_test, y_test)` on a named encoder or selector. A fit on test rows can be meant, and transform raises on a step that was never fitted. The other forms have no fix.

The rule reads the name of the first argument, and for `fit_transform` also the name of the step:

| Call | Name of the first argument | Name of the step |
| --- | --- | --- |
| `fit` | holds `test` | not read |
| `partial_fit` | holds `test` or `holdout`, or is a validation name such as `X_val` | not read |
| `fit_transform` | holds `test` or `holdout`, or is a validation name such as `X_val` | such as `scaler`, `imputer`, `encoder`, `ohe` or `tfidf` |

The rule reads the step's name, because code that refits `PolynomialFeatures` or a step for a plot
does it on purpose.

A refitted step can't move a score on data with no signal, so the forms without a target are
info. With the target, only some steps leaked, so `fit_transform` with a target is info as well.

| Form | Data | Result |
| --- | --- | --- |
| `LogisticRegression` fitted on 60 test rows and scored on them | no signal | 0.997 AUC, against 0.525 when fitted on the training rows |
| five `partial_fit` calls on the test rows | no signal | 0.924 AUC, against 0.518 |
| category_encoders `TargetEncoder` refitted on the test rows with the target | no signal | 0.992 AUC |
| `SelectKBest` refitted on the test rows with the target | no signal | 0.438 AUC |
| scikit-learn `TargetEncoder` refitted on the test rows with the target | no signal | 0.499 AUC |
| `StandardScaler` refitted on batches of 5 test rows | signal | 0.048 AUC lost |
| `OrdinalEncoder` refitted on 30 test rows | signal | 0.058 AUC lost |

The fix scales or codes the test rows with the statistics of the training rows. On the cases built
for it, the fix changed the test matrix by up to 0.60 and some predicted labels, and a test level
that the step hasn't seen now raises. At 4 of 17 held-back sites the fixed code raised, because the scaler
was built again on the line before, or one `LabelEncoder` was last fitted on another column.

The rule as the readers saw it gave wrong advice in 3 and 3 of 23 labelled reports, and the
narrowed rule no longer makes the reports. Recall was 41 of 45 for the error forms and 62 of 85
for the info forms. The rule misses a step whose name it doesn't read, such as
`rs = RobustScaler()` or `imp`, and a test name with no separator, such as `Xtest`. It also
misses held-out rows named `X_dev`, `X_te` or `X_cv`.
