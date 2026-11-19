A shuffled split puts test rows between training rows in time, so a `TimeSeriesRegressor` looks better on them than on later rows. Pass `shuffle=False`.

```diff lang="python"
-X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.3, random_state=0)
+X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.3, shuffle=False)
 series = TimeSeriesRegressor(model, cv=blocks)
 series.fit(X_train, y_train)
```

## What the rule reports

The rule reports `train_test_split` and `train_conformalize_test_split` without `shuffle=False`,
where the first result goes into `fit` of a `TimeSeriesRegressor`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix adds `shuffle=False` at the end of the arguments and drops the comments between them. The parts then hold other rows. A split with `stratify` has no fix, because scikit-learn refuses `stratify` with `shuffle=False`.

On a random walk of 900 rows, the intervals covered the following shares of the test rows:

| Model | Test rows | Coverage |
| --- | --- | --- |
| `TimeSeriesRegressor` | a shuffled test part | 0.921 |
| `TimeSeriesRegressor` | a test part after the training rows | 0.536 |
| `SplitConformalRegressor` on a shuffled split | its own test rows | 0.891 |
| `SplitConformalRegressor` on a shuffled split | the 300 rows that came after them | 0.144 |

The rule can't see that the rows are in time order, so it is a warning.
