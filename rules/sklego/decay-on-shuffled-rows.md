`DecayEstimator` gives the last rows the most weight, and `train_test_split` shuffles the rows, so the weights no longer follow time. Pass `shuffle=False` to the split.

```diff lang="python"
-X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
+X_train, X_test, y_train, y_test = train_test_split(X, y, shuffle=False)
 model = DecayEstimator(Ridge(), decay_kwargs={"decay_rate": 0.99}).fit(X_train, y_train)
```

## What the rule reports

The rule reports `train_test_split` without `shuffle=False` in a file that builds a
`DecayEstimator`. The estimator weights each row by its position, the last row most, and never
reads a date. It is therefore right only on rows in time order.

## How to fix it

With `--unsafe-fixes`, xpd sets `shuffle=False` when the split does not stratify.
Sort the rows by time before splitting them.

On 2,000 rows whose slope changed from 1 to 3 at row 1,000, the model was fitted on the first
1,500 rows and scored on the last 500:

| Order of the 1,500 training rows | Slope | Mean absolute error |
| --- | --- | --- |
| in time order | 3.028 | 0.078 |
| shuffled, as the mean of 30 shuffles | 1.621 | 0.648 |

A pattern can't see which rows the estimator is fitted on, so the rule is a warning.
