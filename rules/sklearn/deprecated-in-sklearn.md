These forms warn FutureWarning on scikit-learn 1.9.1 because a later release removes them or changes their default. Use the replacement that the message names.

```diff lang="python"
-model = LogisticRegression(penalty="l1", solver="liblinear", n_jobs=-1)
-svm = SVC(probability=True, random_state=0).fit(X_train, y_train)
-tree = DecisionTreeRegressor(criterion="friedman_mse", random_state=0)
+model = LogisticRegression(l1_ratio=1, solver="liblinear")
+svm = CalibratedClassifierCV(SVC(), ensemble=False).fit(X_train, y_train)
+tree = DecisionTreeRegressor(criterion="squared_error", random_state=0)
```

## What the rule reports

The rule reports forms that warn FutureWarning on scikit-learn 1.9.1. Its message gives the
release that removes each form or changes its default:

| Form | Removed or changed in | Write instead |
| --- | --- | --- |
| `SVC(probability=True)`, `NuSVC(probability=True)` | 1.11 | `CalibratedClassifierCV(SVC(), ensemble=False)` |
| `probability=False` on `SVC` and `NuSVC` | 1.11 | nothing, because it is the default |
| `probA_`, `probB_` | 1.11 | the calibrators of `CalibratedClassifierCV(ensemble=False)` |
| `LogisticRegression(penalty=...)` | 1.10 | `l1_ratio`, 0 for L2, 1 for L1 and a value between for elasticnet, and `C=np.inf` for no penalty |
| `LogisticRegression(l1_ratio=None)` | 1.10 | `l1_ratio=0` |
| `LogisticRegression(n_jobs=...)` | 1.10 | nothing, because it has had no effect since 1.8 |
| `LogisticRegressionCV(penalty=...)` | 1.10 | `l1_ratios`, `(0,)` for L2 and `(1,)` for L1 |
| `LogisticRegressionCV` without `l1_ratios`, `scoring` and `use_legacy_attributes` | 1.10 and 1.11 | all three set |
| `criterion="friedman_mse"` on regression trees and forests | 1.11 | `criterion="squared_error"` |
| `criterion=` on GradientBoosting | 1.11 | nothing, because it has no effect |
| `PassiveAggressiveClassifier`, `PassiveAggressiveRegressor` | 1.10 | `SGDClassifier(loss="hinge", penalty=None, learning_rate="pa1", eta0=1.0)`, or `SGDRegressor` with `loss="epsilon_insensitive"` |
| `TargetEncoder(shuffle=, random_state=)` | 1.11 | a splitter as `cv`, such as `cv=KFold(5, shuffle=True, random_state=0)` |
| `MDS(dissimilarity=)` | 1.10 | `metric=` |
| `MDS(metric=True)` or `metric=False` | 1.10 | `metric_mds=` |
| `MDS` without `init` | 1.10 | `init="random"` to keep the current result |
| `HDBSCAN` without `copy` | 1.10 | `copy=False` to keep the current behavior, or `copy=True` |
| `n_alphas=` on the path functions | 1.11 | `alphas=` |
| `alphas=None` | 1.11 | `alphas=100` |
| a negative `power_t` | 1.10 | a value of 0 or more |
| `sklearn.utils.extmath.stable_cumsum` | 1.10 | `np.cumulative_sum` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix drops `probability=False` from `SVC` and `NuSVC`, drop `penalty="elasticnet"` beside an `l1_ratio`, drop `n_jobs=` from `LogisticRegression`, `criterion="squared_error"` for `"friedman_mse"` on regression trees and forests, drop `criterion=` from GradientBoosting, `metric=` for MDS `dissimilarity=`, `metric_mds=` for MDS `metric=True` or `False`, `alphas=` for `n_alphas=`, `alphas=` for `n_alphas=` beside `alphas=None`, `alphas=100` for `alphas=None`, `C=np.inf` for `penalty=None` on `LogisticRegression`, and `init="random"` for MDS without `init`. Unsafe: drop `penalty="l2"`, `l1_ratio=1` for `penalty="l1"`, and `l1_ratio=0` for `l1_ratio=None` on `LogisticRegression`. Also unsafe: `CalibratedClassifierCV(SVC(), ensemble=False)` for `probability=True`, whose probabilities and predictions differ, `l1_ratios=(0,)` or `(1,)` for `penalty="l2"` or `"l1"` on `LogisticRegressionCV`, and `l1_ratios=(0,), scoring="accuracy", use_legacy_attributes=True` on a `LogisticRegressionCV` that sets none of the three. There `l1_ratio_`, `scores_`, `coefs_paths_` and `n_iter_` change form. A passive aggressive model without `C` or `loss` becomes `SGDClassifier` or `SGDRegressor` with `penalty=None`, `learning_rate="pa1"` and `eta0=1.0`, which fits the same model as another class. The other forms have no fix.

The proof ran 30 forms on scikit-learn 1.9.1. Under `-W error` each old form raised its
FutureWarning, and its new form ran clean. `LogisticRegression(n_jobs=None)` gives no warning and is
left alone.

Every safe fix gave a bit equal fitted model and bit equal predictions in every run. A fix that
drops a keyword changes the stored constructor parameter, which is not counted as a change of the
model.

| Safe fix | Runs |
| --- | --- |
| dropping `probability=False` | 16 |
| dropping `penalty="elasticnet"` | 16 |
| dropping `n_jobs` | 38 |
| `criterion="squared_error"` for `"friedman_mse"` | 16 |
| dropping the GradientBoosting `criterion` | 6 |
| the two MDS renames | 4 and 6 |
| `alphas=` for `n_alphas=` | 8 |
| `alphas=100` for `alphas=None` | 6 |

The three unsafe fixes were bit equal in 48 runs, 32 runs and 8 runs. They differ only with
`C=np.inf`, which 1.9.1 turns into no penalty only when `penalty` is left at its default:

| Unsafe fix | With `C=np.inf` |
| --- | --- |
| `l1_ratio=1` for `penalty="l1"`, with liblinear | The fixed form raises where the old form ran. |
| dropping `penalty="l2"`, with liblinear | The fixed form raises ValueError where the old form didn't finish in 60 s. |
| `l1_ratio=0` for `l1_ratio=None`, with saga | The fixed form runs where the old form raised TypeError. |

Of 101 held-back reports, 57 were labelled, and reader A called 3 wrong. Recall was 274 of 308. Of
the misses, 12 were `sklearn.metrics.cluster.entropy`, which
[`deprecated-metric-api`](deprecated-metric-api.md) reports. The rule also misses `HDBSCAN`,
`stable_cumsum` and the PassiveAggressive classes reached through a module or another name, and a
deprecated value in a parameter grid or a variable.
