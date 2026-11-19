# MAPIE

The MAPIE rules report the MAPIE 0.x API, which raises on MAPIE 1, code that breaks the coverage
that a conformal method promises, arguments that MAPIE refuses, and code that runs the model twice,
in each file that imports `mapie`. [MAPIE](https://mapie.readthedocs.io/) builds prediction
intervals and prediction sets around a scikit-learn model with conformal methods, which promise
that the intervals or sets cover a chosen share of new rows. Every number about the MAPIE rules was
measured on MAPIE 1.6.0 with scikit-learn 1.9.1. The examples assume that the MAPIE and
scikit-learn names they use are imported, and names such as `X_train`, `X_conf`, `y_test` and
`model` stand for values from your program.

MAPIE 1.0 changed the API:

| MAPIE 0.x | MAPIE 1 |
| --- | --- |
| `MapieRegressor`, `MapieClassifier` | one class per method |
| `alpha` | `confidence_level` on the constructor, which is `1 - alpha` |
| `fit` | `fit` and `conformalize` |

A rule document about the change carries `since`. xpd reads the installed version from the
project's venv and skips the document on an older release, so a project that still has MAPIE 0.9
gets no report about the 0.x API. When the venv has no MAPIE, or there is no venv, every document
runs.

Some rules read the names of variables, such as `X_train`, because xpd reads the code and never
the values it holds. A name that misleads therefore gives a wrong report. The other rules tie a call to an object by the
variable the file assigns it to, and to rows by the variable given to an earlier call.

## The proof each rule needs

Each rule document ships only with a proof that was run. A proof is one of the following kinds:

| Proof | What it does |
| --- | --- |
| coverage | Runs the old and the new form over 30 seeds with `confidence_level=0.9`, and gives the mean share of test rows that the intervals or sets cover. |
| interpreter | Runs the old form on MAPIE 1.6.0, under `python -W error` for a warning. It passes when the old form raises and the new form runs clean. |
| speed | Times both forms on one thread, as the median of 5 or 7 runs, at two data sizes, and compares the arrays they return. |
| rerun | Runs the old form twice in fresh objects. It passes when the two results differ and the new form gives the same result twice. |

A document is `error` only on a coverage proof or an interpreter proof, and its message quotes
what was measured. A document that can also match code written that way on purpose is `warning`, such as a level
below 0.5 in a plot of coverage against the level.

The rules were then run over MAPIE's own code at the 1.6.0 tag, and every report was read. The
code was its 162 Python files, which hold the library, its tests and its examples, and the Python
code of its 21 documentation pages and notebooks. A rule that reported correct code there was
narrowed, and the table gives what is left.

| Rule | Severity | Proof | Reports on MAPIE's own code | Wrong advice | Fix |
| --- | --- | --- | --- | --- | --- |
| [`removed-in-mapie`](removed-in-mapie.md) | error | interpreter: ImportError, ModuleNotFoundError, TypeError, ValueError, AttributeError | 1 | 0 | unsafe |
| [`deprecated-in-mapie`](deprecated-in-mapie.md) | warning | interpreter: FutureWarning, and intervals equal with and without `gamma` | 2 | 0 | safe |
| [`alpha-as-confidence-level`](alpha-as-confidence-level.md) | warning, error | coverage: 0.108 against 0.907, and interpreter: ValueError | 0 | no reports | unsafe |
| [`calibrate-on-training-rows`](calibrate-on-training-rows.md) | error | coverage: 0.468 against 0.907 | 6 | 1 | none |
| [`coverage-on-seen-rows`](coverage-on-seen-rows.md) | error, warning | coverage: 1.000 against 0.913 | 0 | no reports | none |
| [`shuffled-time-series-split`](shuffled-time-series-split.md) | warning | coverage: 0.921 against 0.536 | 0 | no reports | unsafe |
| [`prefit-estimator-not-fitted`](prefit-estimator-not-fitted.md) | error | interpreter: NotFittedError, AttributeError, ValueError | 0 | no reports | none |
| [`quantile-estimator-loss`](quantile-estimator-loss.md) | error | interpreter: ValueError | 0 | no reports | unsafe |
| [`predict-function-returns-labels`](predict-function-returns-labels.md) | error | interpreter: IndexError | 0 | no reports | unsafe |
| [`risk-control-method`](risk-control-method.md) | error, warning | interpreter: ValueError, UserWarning | 0 | no reports | none |
| [`predict-and-predict-interval`](predict-and-predict-interval.md) | warning | speed: 1.97 times | 3 | 3 | none |
| [`refit-per-confidence-level`](refit-per-confidence-level.md) | warning | speed: 2.87 times | 1 | 1 | none |
| [`unseeded-resampling`](unseeded-resampling.md) | info | rerun: the result differs | 36 | 0 | none |

A coverage proof gives the old form's mean against the new form's mean. A speed proof gives the
lowest ratio over the forms the rule reports and both sizes. Wrong advice counts the reports where
following the message would not fix a problem in the code.

All 5 reports with wrong advice are in MAPIE's tests:

- Three tests call `predict` beside `predict_interval` or `predict_set` to assert that the two
  agree.
- One test builds one object per level to compare it with the list form.
- One test conformalizes on the rows of a `DummyRegressor` that predicts a constant, which learns
  nothing from them.

Four more reports of `calibrate-on-training-rows` are tests that fit and conformalize on the same
rows to check the mechanics and don't measure coverage. The sixth is an example in MAPIE's
gallery that conformalizes on its training rows. No rule reported the library's own source, and
the examples, documentation pages and notebooks got 9 reports, none with wrong advice.

Recall was not measured, because no MAPIE code from other projects was held back. The table
therefore says how often a rule is wrong when it reports, and not how often it misses. The pack has no rule for
`mapie.exchangeability_testing`, `mapie.experimental` and the FWER procedures. Their misuse is in
the data, such as rows shuffled before an exchangeability test, and a pattern can't see it.

## The 0.x API

Each rule in the group reports a form that raises or warns on MAPIE 1.6.0. Each form was run
there, and the messages take each replacement from the v1 release notes or from the error text. A
fix of a removed form turns code that raises into code that runs, so it can't be compared with the
old form and is never safe.

- removed-in-mapie
- deprecated-in-mapie

## Coverage that does not hold

Each rule in the group reports code that breaks the coverage a conformal method promises. The
promise is that the intervals cover a share of new rows, and it needs the following conditions:

- conformity scores from rows the model did not learn from
- a level that is the share to cover
- test rows that the method has not seen

Each number below is the mean share of test rows covered over the seeds 0 to 29, with a
`RandomForestRegressor` of 50 trees unless another model is named. The regression data are 1,000
rows of 5 normal columns, with a target of the first column, the sine of the second and normal
noise, split into 60, 20 and 20 percent by `train_conformalize_test_split`.

- alpha-as-confidence-level
- calibrate-on-training-rows
- coverage-on-seen-rows
- shuffled-time-series-split

## Arguments that raise

Each rule in the group reports a form that raises when the object is used on MAPIE 1.6.0. Code
inside `pytest.raises` or `pytest.warns` is left alone.

- prefit-estimator-not-fitted
- quantile-estimator-loss
- predict-function-returns-labels
- risk-control-method

## Work done twice

Take the points and the intervals from one call, and pass every confidence level to one estimator, because each extra call repeats the same work.

- predict-and-predict-interval
- refit-per-confidence-level

## Reproducible results

Pass `random_state` to each MAPIE resampler, so two runs give the same intervals.

- unseeded-resampling
