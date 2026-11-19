`RegressionOutlierDetector` with a column name keeps the target among the features, so the inner model predicts the target from itself. Pass the column's position on an array instead.

```diff lang="python"
-detector = RegressionOutlierDetector(LinearRegression(), column="y").fit(frame)
+position = frame.columns.get_loc("y")
+detector = RegressionOutlierDetector(LinearRegression(), column=position).fit(frame.to_numpy())
```

## What the rule reports

The rule reports `RegressionOutlierDetector(model, column="y")`, where the column is a name. The
detector finds the target of a frame by name, and then drops the column whose position equals
`column` from the features. A name equals no position, so the target stays among the features and
the inner model predicts it from itself.

## How to fix it

xpd has no automatic fix, so make the change by hand.

On 500 rows with 10 planted outliers, the two forms gave the following fits:

| Form | Features | Coefficients | Spread of the residuals | Rows flagged |
| --- | --- | --- | --- | --- |
| `column="y"` on the frame | 3 | 0, 0 and 1 | 9.50e-16 | 11 |
| `column=2` on `frame.to_numpy()` | 2 | not given | 3.25 | the 10 planted rows and no other |

A position on a frame is wrong in another way, which a pattern can't see. The detector compares the
frame's column names with the number and finds no match, so the first column becomes the target.
scikit-lego's own test of the name form passes because the planted rows have the largest rounding
errors.
