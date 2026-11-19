# scikit-lego

The scikit-lego rules report arguments that give a wrong number without an error, steps that read
the wrong rows or can't be fitted, and code that raises or warns on scikit-lego 0.9.10, in each
file that imports `sklego`. Every number about the scikit-lego rules was measured on scikit-lego 0.9.10 with
scikit-learn 1.9.1, NumPy 2.5.5 and pandas 3.1.1. The examples assume that the names they use are
imported, and names such as `X_train`, `y_test` and `model` stand for values from your program.

A rule document whose advice needs a newer release of scikit-lego carries `since`. xpd reads the
installed version from the project's venv and skips the document on an older release. When the
venv has no scikit-lego, or there is no venv, every document runs.

The pack has no package checks. cvxpy, umap-learn and formulaic are extras that only some
estimators need, and a package check can't see which estimators a project uses. When an extra is
missing, scikit-lego raises an ImportError that names it.

## The proof each rule needs

A rule is an `error` only when code was run that shows the wrong number or the exception, and
its message quotes what was measured. A proof is one of the following kinds:

| Proof | What runs |
| --- | --- |
| number | The old and the new form on the same data, and a comparison of what they return. |
| interpreter | The old form, with its exception or its warning under `python -W error`, and the new form, which runs clean. |
| speed | The old form, timed at two settings. |

A rule is a `warning` when the proof passed and a pattern can't see whether the code wants the old
form. A rule is `info` when no proof of the listed kinds applies.

The rules were then run over scikit-lego's own source, docs scripts and tests at the 0.9.10 tag,
134 Python files, and every report was read. Of the 46 reports there, 42 are on a form that the
rule describes, most of them in tests that build a deprecated or superseded class or check that a
wrong call raises. The other 4 are on code that is right as it stands:

- the package's own export of `PandasTypeSelector`
- two tests of `add_lags` with a positive lag
- one test that passes `threshold=0.1` to `GMMOutlierDetector` only to build an object

After that reading, `debug-pipeline-default-log` was narrowed to leave a file alone when it
imports `logging` or uses pytest's `caplog`, which removed 5 reports. `unseeded-random-estimator`
was narrowed to leave test files alone, which removed 7.

| Rule | Severity | Proof | Reports on scikit-lego's files | On code that is right | Fix |
| --- | --- | --- | --- | --- | --- |
| [`thresholder-score`](thresholder-score.md) | error | number: 0.887 at three thresholds against 0.740, 0.887 and 0.847 | 0 | 0 | none |
| [`estimator-transformer-proba`](estimator-transformer-proba.md) | error | number: shape (800, 1) for 400 rows | 0 | 0 | none |
| [`column-capper-fraction`](column-capper-fraction.md) | error | number: 9,910 of 10,000 values changed against 1,000 | 0 | 0 | none |
| [`dict-mapper-not-int`](dict-mapper-not-int.md) | error | number: 0.5 and 1.7 became 0 and 1. Interpreter: ValueError | 0 | 0 | none |
| [`regression-outlier-column-name`](regression-outlier-column-name.md) | error | number: coefficients 0, 0 and 1, and 11 rows flagged against 10 | 2 | 0 | none |
| [`pca-outlier-after-scaler`](pca-outlier-after-scaler.md) | error | number: 320 rows flagged against 20 | 0 | 0 | none |
| [`outlier-threshold`](outlier-threshold.md) | error | interpreter: ValueError. Number: 380 of 400 rows flagged against 20 | 1 | 1 | none |
| [`grouped-shrinkage-arguments`](grouped-shrinkage-arguments.md) | error | number: probabilities bit equal with and without the keyword. Interpreter: TypeError | 0 | 0 | unsafe |
| [`add-lags-positive`](add-lags-positive.md) | warning | number: accuracy 0.996 against 0.474 on noise | 2 | 2 | none |
| [`decay-on-shuffled-rows`](decay-on-shuffled-rows.md) | warning | number: slope 1.621 against 3.028 | 0 | 0 | none |
| [`outlier-remover-before-model`](outlier-remover-before-model.md) | error | interpreter: ValueError | 0 | 0 | none |
| [`decay-without-sample-weight`](decay-without-sample-weight.md) | error | interpreter: ValueError, TypeError | 0 | 0 | none |
| [`zero-inflated-roles`](zero-inflated-roles.md) | error | interpreter: ValueError | 2 | 0 | none |
| [`scorer-called-as-metric`](scorer-called-as-metric.md) | error | interpreter: IndexError, TypeError. Number: a function where a score is expected | 0 | 0 | unsafe |
| [`dataset-frame-and-xy`](dataset-frame-and-xy.md) | error | interpreter: ValueError | 0 | 0 | none |
| [`log-step-extra-bare`](log-step-extra-bare.md) | error | number: a function where a frame is expected. Interpreter: ValueError | 1 | 0 | none |
| [`selector-without-feature-names`](selector-without-feature-names.md) | error | interpreter: ValueError, AttributeError | 0 | 0 | none |
| [`debug-pipeline-default-log`](debug-pipeline-default-log.md) | info | number: 0 characters printed | 0 | 0 | none |
| [`removed-in-sklego`](removed-in-sklego.md) | error | interpreter: ImportError, ModuleNotFoundError, TypeError | 0 | 0 | unsafe |
| [`deprecated-in-sklego`](deprecated-in-sklego.md) | warning | interpreter: DeprecationWarning | 10 | 1 | safe |
| [`superseded-estimator`](superseded-estimator.md) | warning, info | interpreter: UserWarning | 28 | 0 | none |
| [`group-time-series-split-folds`](group-time-series-split-folds.md) | warning | speed: 1.08 s at 5 folds against 10.48 s at 6 | 0 | 0 | none |
| [`unseeded-random-estimator`](unseeded-random-estimator.md) | info | none of the three | 0 | 0 | none |

A rule with no report on scikit-lego's files was checked only on the pack's sample files, so its
rate of wrong reports on real code is not measured. The pack was not judged on held-back files
from other projects, and it has no recall number.

## Arguments that give a wrong number

Each rule in the group reports an argument that gives another number than the code asks for,
without an error, or that raises where the argument is read.

- thresholder-score
- estimator-transformer-proba
- column-capper-fraction
- dict-mapper-not-int
- regression-outlier-column-name
- pca-outlier-after-scaler
- outlier-threshold
- grouped-shrinkage-arguments

## Rows and fits

Each rule in the group reports a step fitted on rows it shouldn't see in that order, or a meta
estimator built around a model it can't fit.

- add-lags-positive
- decay-on-shuffled-rows
- outlier-remover-before-model
- decay-without-sample-weight
- zero-inflated-roles

## Scorers, datasets and helpers

Call each scikit-lego scorer, dataset loader and pipeline helper with the arguments it reads, because the forms below raise or return something other than the code expects.

- scorer-called-as-metric
- dataset-frame-and-xy
- log-step-extra-bare
- selector-without-feature-names
- debug-pipeline-default-log

## Removed, deprecated and superseded forms

Each rule in the group reports a form that raises or warns on scikit-lego 0.9.10, and each removed
form raised when it was run. The deprecated and superseded forms were run under `python -W error`,
where the old form raised its warning and the new form ran clean. The messages take each
replacement from the text of the warning or from the release that made the change. A fix of a
removed form turns code that raises into code that runs, so no such fix is safe.

- removed-in-sklego
- deprecated-in-sklego
- superseded-estimator

## Speed and repeatable runs

Keep `GroupTimeSeriesSplit` to few folds, and give each estimator that draws random numbers a seed, so a run finishes and repeats.

- group-time-series-split-folds
- unseeded-random-estimator

## Habits the pack does not report

The pack has no rule for the following habits:

- `MaximumRelevanceMinimumRedundancy` with the default `kind="auto"` scores a whole-number
  regression target as classes. On one such target it picked the features 0, 1 and 3, against 0, 1
  and 4 with `kind="regression"`. A pattern can't see the target, and a rule on every call without
  `kind` would report the library's own tests.
- `RegressionOutlierDetector` with a column position on a frame, and `TimeGapSplit`, `RandomAdder`
  and `OutlierRemover` given an input type they don't take, depend on what `fit` is given, which a
  pattern can't see.
- The extras cvxpy, umap-learn and formulaic have no rule, because a package check can't see which
  estimators a project uses.
