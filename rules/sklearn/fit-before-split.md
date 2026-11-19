A step that learned from the target on every row before the split lets the test rows choose what the model sees, so the test score is too high. Split first, fit the step on the training part, and transform both parts.

```diff lang="python"
-X = selector.fit_transform(X, y)
 X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
+X_train = selector.fit_transform(X_train, y_train)
+X_test = selector.transform(X_test)
```

## What the rule reports

The rule reports a step that learns from the target and is fitted on every row before
`train_test_split`, such as `selector.fit_transform(X, y)`, `encoder.fit(X, y).transform(X)`,
`fit_resample`, `resample(..., replace=True)`, a target mean per group from
`groupby(...)[target].transform("mean")`, and columns picked by `df.corr()[target]`.

The rule leaves alone a label encoder on the target, when the result is named `y`, `target` or
`labels`, or the encoder's name holds `label`. The target mean and the `df.corr()` forms need the
same column to become the target later in the block, as in `y = df["target"]`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The rule reports the fit when a later statement of the same block passes its result to
`train_test_split`. On data with no signal, the steps gave the following mean test AUC:

| Step fitted on every row | Before the split, against after | Reported |
| --- | --- | --- |
| `SelectKBest` | 0.771 against 0.497 | yes |
| `SelectFromModel` | 0.684 against 0.519 | yes |
| `RFE` | 0.750 against 0.495 | yes |
| `RFECV` | 0.739 against 0.506 | yes |
| `SequentialFeatureSelector` | 0.551 against 0.477 | yes |
| category_encoders `TargetEncoder` | 0.880 against 0.495 | yes |
| `TargetEncoder().fit(X, y).transform(X)` | 0.889 against 0.509 | yes |
| a target mean per group | 0.887 against 0.497 | yes |
| 10 columns picked by `df.corr()` | 0.771 against 0.497 | yes |
| `SMOTE` | 0.815, against 0.478 to 0.493 for the samplers after the split | yes |
| `RandomOverSampler` | 0.737 | yes |
| `ADASYN` | 0.819 | yes |
| `EditedNearestNeighbours` | 0.678 | yes |
| `resample(replace=True)` | 0.970 | yes |
| `RandomUnderSampler` | 0.512 against 0.491 | no, it only drops rows |
| `NearMiss` | 0.448 | no, it only drops rows |
| `TomekLinks` | 0.516 | no, it only drops rows |
| scikit-learn `TargetEncoder.fit_transform`, which cross-fits | 0.522 against 0.504 | no |

The rule as the readers saw it gave wrong advice in 2 and 0 of 26 labelled reports. Recall was 91
of 162. The rule found none of 24 columns picked by correlation in other shapes, such as
`corr.index[1:11]` read back as `df[top_features]`. It also misses a target mean written with
`.map` or `.mean()`, `resample` imported under another name, and `SelectFromModel` fitted in one
statement and applied in the next.
