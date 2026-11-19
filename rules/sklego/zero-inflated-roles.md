`ZeroInflatedRegressor` takes the classifier first and the regressor second, so swapping them raises ValueError at fit. Pass a classifier first.

```diff lang="python"
-model = ZeroInflatedRegressor(LinearRegression(), LogisticRegression())
+model = ZeroInflatedRegressor(classifier=LogisticRegression(), regressor=LinearRegression())
```

## What the rule reports

The rule reports `ZeroInflatedRegressor` with a class named like a regressor as the classifier, or
a class named like a classifier as the regressor. The classifier comes first and decides whether a
value is zero, and the regressor comes second.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`fit` raised ValueError "`classifier` has to be a classifier", and with a classifier in second
place it raised "`regressor` has to be a regressor". The rule reads the class names, and it knows
that `LogisticRegression` is a classifier.
