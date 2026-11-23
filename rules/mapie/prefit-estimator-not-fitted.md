These classes default to `prefit=True`, and an estimator built in the call is not fitted, so `conformalize` raises. Pass `prefit=False` and call `fit` first, or fit the estimator before you pass it.

```diff lang="python"
-mapie = SplitConformalRegressor(RandomForestRegressor(random_state=0), confidence_level=0.9)
+mapie = SplitConformalRegressor(RandomForestRegressor(random_state=0), confidence_level=0.9, prefit=False)
 mapie.fit(X_train, y_train)
```

## What the rule reports

The rule reports `SplitConformalRegressor` and `SplitConformalClassifier` with an estimator built
in the argument list and no `prefit=False`, and `fit` on such an object without `prefit=False`. It
also reports `ConformalizedQuantileRegressor(prefit=True)` with one estimator.

## How to fix it

`xpd check --fix --unsafe-fixes` adds `prefit=False` on inline chained `SplitConformalRegressor(...).fit(...)` and `SplitConformalClassifier(...).fit(...)` calls. Other forms remain manual because fitting the estimator beforehand is also a valid fix.

`prefit` is `True` by default on the split classes, which says the estimator is already fitted.
With an estimator built in the argument list, each call raised:

| Call | Error |
| --- | --- |
| `conformalize`, with a regressor | NotFittedError |
| `conformalize`, with a classifier | "Invalid classifier. Fitted classifier does not contain 'classes_' attribute" |
| `fit` | "The fit method must be skipped when the prefit parameter is set to True" |

The rule reads a class name or `make_pipeline` as a new estimator, so `LinearRegression().fit(X, y)`
and a model loaded from a file are left alone. A wrapper around a fitted model is fitted too.
`SplitConformalRegressor(FrozenEstimator(fitted))` ran, so `FrozenEstimator` is left alone. A class
that is given a `.fit(...)` chain is left alone too. `fit` is reported on an object that the file assigns
from a split class without `prefit=False` and from nothing else.

`ConformalizedQuantileRegressor` has `prefit=False` as its default. With `prefit=True` it needs
three fitted estimators, for the lower quantile, the upper quantile and the median. A
`QuantileRegressor` or a `HistGradientBoostingRegressor` alone raised "Estimator for prefit must
be an iterable object". A `GradientBoostingRegressor` alone, which can be iterated over its trees,
raised AttributeError when it was not fitted and "You need to have provided 3 different
estimators" when it was. The rule reads only a class built in the argument list, with or without a `fit`
chained to it, as one estimator. A call of a function may return the list of three, and
`ConformalizedQuantileRegressor(fit_three(X, y), prefit=True)` ran.

The conditional classes `ConditionalSplitConformalRegressor` and
`ConditionalSplitConformalClassifier` raised the same errors, and the rule reads them by their
`estimator` keyword.
