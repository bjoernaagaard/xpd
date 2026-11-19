These MAPIE objects draw folds or samples with a new seed on every run, so the intervals change from run to run. Pass `random_state`.

```diff lang="python"
-cross = CrossConformalRegressor(model, confidence_level=0.9, cv=5)
+cross = CrossConformalRegressor(model, confidence_level=0.9, cv=5, random_state=0)
 X_train, X_conf, X_test, y_train, y_conf, y_test = train_conformalize_test_split(
-    X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2
+    X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2, random_state=0
 )
```

## What the rule reports

The rule reports objects that draw random numbers and get no `random_state`. They are
`CrossConformalRegressor`, `CrossConformalClassifier` and `CrossConformalizedQuantileRegressor` with
a number of folds, `JackknifeAfterBootstrapRegressor`, `Subsample`, `BlockBootstrap`,
`train_conformalize_test_split`, and `SplitConformalClassifier` with the `"aps"` or `"raps"` score.

The rule leaves alone:

## How to fix it

xpd has no automatic fix. The seed is not in the file. Make the change by hand.

Any fixed number makes the run repeatable, and 0 is only the example.

The rule is info, because a seed doesn't move the expected coverage. Each form it names gave
another result in two fresh runs and the same result with `random_state`.

The cross classes shuffle their folds when `cv` is a number, which is the default. They gave the
same intervals with `cv=KFold(5)` and with `cv=-1`, so a splitter and a negative number are left
alone. `SplitConformalClassifier` gave the same sets twice with the `"lac"` and `"top_k"` scores.
It is reported only with `"aps"` and `"raps"`, which draw a random number for each row.

- a file that calls `np.random.seed`, after which the same objects gave the same result twice
- a call with `shuffle=False` or `**`
- a `JackknifeAfterBootstrapRegressor` whose `resampling` is a `Subsample`, which is reported
  itself
- a call whose result is thrown away

A splitter that shuffles draws its own seed, and `random_state` of the cross class doesn't reach
it. `CrossConformalRegressor(model, cv=KFold(5, shuffle=True), random_state=0)` gave other
intervals in two runs, and the same intervals with `KFold(5, shuffle=True, random_state=0)`. The
rule doesn't report such a splitter, because the scikit-learn rule
[`missing-random-state`](../sklearn/missing-random-state.md) reports the estimator and a `KFold` with
`shuffle=True` in the same file.
