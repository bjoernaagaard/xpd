# scikit-learn

The scikit-learn rules report code that gives a leaked or wrong score, code that raises or warns
on scikit-learn 1.9.1, and code that does the same work twice, in each file that imports
`sklearn`. Every number about the scikit-learn rules was measured on scikit-learn 1.9.1. The examples assume
that the scikit-learn names they use are imported, and names such as `X_train`, `y_test` and
`model` stand for values from your program.

Several rules read the names of variables, such as `X_test` and `y_pred`, because xpd reads the
code and never the values it holds. A name that misleads therefore gives a wrong report.

A rule document whose advice needs a newer release of scikit-learn carries `since`. xpd reads the
installed version from the project's venv and skips the document on an older release. When the
venv has no scikit-learn, or there is no venv, every document runs.

## The proof each rule needs

Each rule document ships only with a saved proof that its old form is wrong. A proof is one of
the following kinds:

| Proof | What runs | When it passes |
| --- | --- | --- |
| score | The old and the new form, over 30 seeds on data with no signal, where the honest score is 0.5. | The old form's mean is more than three standard errors above the honest score, and the new form's mean is within 0.02 of it. |
| interpreter | The old form under `python -W error`. | The old form raises an exception or a warning, and the new form runs clean. |
| speed | Both forms on one thread, as the median of 7 runs, at two data sizes. | The new form is at least 1.5 times faster at both sizes. |

A document is `info` when no proof passed for it. Such a document either changes a score in a
way the proofs can't show, or changes nothing that can be measured. For example, a ranking metric
given class labels scores 0.5 on data with no signal in both forms, and a missing seed changes
nothing that can be measured.

The rules were then judged on 1,414 held-back files from 268 public repositories, which were set
aside before any rule was written. The judging had the following parts:

1. Two readers who did not write the rules labelled the same 478 reports, each without the
   other, and marked every report whose advice was wrong.
2. Recall was measured by planting 2,749 old forms in copies of the held-back files and counting
   the plants that a rule reported.
3. Every fix was run before and after, on its held-back sites and on cases built for each form,
   and the fitted model and its predictions were compared bit for bit.

| Rule | Severity | Proof | Held-back reports | Wrong advice | Recall | Fix |
| --- | --- | --- | --- | --- | --- | --- |
| [`fit-on-test`](fit-on-test.md) | error, info | score: 0.997 against 0.525 AUC | 20 | 0 and 0 of 20 | 103 of 130 | unsafe |
| [`fit-before-split`](fit-before-split.md) | error | score: 0.771 against 0.497 AUC | 24 | 0 and 0 of 24 | 91 of 162 | none |
| [`cross-val-on-fitted-data`](cross-val-on-fitted-data.md) | error | score: 0.764 against 0.518 AUC | 10 | 0 and 0 of 10 | 72 of 78 | none |
| [`tune-on-test`](tune-on-test.md) | error | score: 0.597 against 0.507 accuracy | 1 | 0 and 0 of 1 | 15 of 50 | none |
| [`stateful-function-transformer`](stateful-function-transformer.md) | info | none of the three | 0 | no reports | 24 of 30 | none |
| [`groups-ignored`](groups-ignored.md) | error | score: 0.849 against 0.480 accuracy | 0 | no reports | 56 of 57 | unsafe |
| [`missing-groups`](missing-groups.md) | error | interpreter: ValueError | 0 | no reports | 48 of 50 | none |
| [`leave-one-out-score`](leave-one-out-score.md) | error | interpreter: UndefinedMetricWarning | 0 | no reports | 14 of 33 | none |
| [`auc-from-labels`](auc-from-labels.md) | error, info | score: log loss 18.00 against 0.70 | 20 | 2 and 2 of 20 | 55 of 72 | unsafe |
| [`metric-arguments-swapped`](metric-arguments-swapped.md) | error, info | score: precision 0.471 against 0.294 | 43 | 0 and 0 of 30 | 66 of 100 | unsafe |
| [`score-on-training-rows`](score-on-training-rows.md) | error | score: 1.000 against 0.510 accuracy | 0 | no reports | 10 of 27 | none |
| [`unknown-scoring-name`](unknown-scoring-name.md) | error | interpreter: InvalidParameterError, ValueError | 4 | 0 and 0 of 4 | 16 of 32 | none |
| [`removed-in-sklearn`](removed-in-sklearn.md) | error | interpreter: ModuleNotFoundError, ImportError, TypeError, InvalidParameterError | 321 | 1 and 0 of 58 | 517 of 600 | unsafe |
| [`deprecated-in-sklearn`](deprecated-in-sklearn.md) | warning | interpreter: FutureWarning | 101 | 3 and 0 of 57 | 274 of 308 | safe, unsafe |
| [`removed-metric-api`](removed-metric-api.md) | error | interpreter: ImportError, TypeError | 13 | 0 and 0 of 13 | 74 of 74 | unsafe |
| [`deprecated-metric-api`](deprecated-metric-api.md) | warning | interpreter: FutureWarning | 0 | no reports | 48 of 48 | safe |
| [`fit-params-keyword`](fit-params-keyword.md) | error | interpreter: TypeError | 4 | 0 and 0 of 4 | 32 of 32 | unsafe |
| [`column-transformer-removed`](column-transformer-removed.md) | error | interpreter: TypeError | 3 | 0 and 0 of 3 | 30 of 30 | unsafe |
| [`metadata-routing`](metadata-routing.md) | error | interpreter: RuntimeError, ValueError, TypeError, UnsetMetadataPassedError | 0 | no reports | 34 of 34 | unsafe |
| [`pipeline-fit-params`](pipeline-fit-params.md) | error | interpreter: ValueError | 0 | no reports | 23 of 30 | none |
| [`pipeline-step-form`](pipeline-step-form.md) | error | interpreter: ValueError, TypeError | 0 | no reports | 29 of 35 | unsafe |
| [`transformed-target-pair`](transformed-target-pair.md) | error | interpreter: ValueError | 0 | no reports | 31 of 36 | unsafe |
| [`pandas-output-sparse`](pandas-output-sparse.md) | error | interpreter: ValueError | 0 | no reports | 29 of 31 | unsafe |
| [`predict-per-row`](predict-per-row.md) | warning | speed: 2.77 times | 2 | 0 and 2 of 2 | 28 of 32 | none |
| [`score-after-predict`](score-after-predict.md) | warning | speed: 1.62 times | 17 | 0 and 0 of 17 | 31 of 32 | unsafe |
| [`predict-and-predict-proba`](predict-and-predict-proba.md) | warning | speed: 1.83 times | 22 | 0 and 0 of 22 | 30 of 30 | none |
| [`cross-val-score-per-metric`](cross-val-score-per-metric.md) | warning | speed: 1.60 times | 14 | 0 and 0 of 14 | 30 of 30 | none |
| [`vectorizer-fit-then-transform`](vectorizer-fit-then-transform.md) | warning | speed: 1.63 times | 12 | 0 and 4 of 12 | 19 of 25 | unsafe |
| [`refit-per-n-estimators`](refit-per-n-estimators.md) | warning | speed: 1.64 times | 0 | no reports | 30 of 30 | none |
| [`tsne-exact`](tsne-exact.md) | info | none of the three | 0 | no reports | 21 of 30 | unsafe |
| [`random-state-without-shuffle`](random-state-without-shuffle.md) | error | interpreter: ValueError | 9 | 0 and 0 of 9 | 54 of 54 | none |
| [`missing-random-state`](missing-random-state.md) | info | none of the three | 527 | 1 and 1 of 52 | 79 of 80 | none |

| Column | What it gives |
| --- | --- |
| Proof | A score proof gives the old form's mean against the new form's mean. A speed proof gives the lowest ratio over the classes the rule reports and both sizes. |
| Wrong advice | The reports that reader A and reader B called wrong, out of the reports they labelled. A rule with more reports than labels was sampled. A rule with no report on the held-back files has no number. |
| Recall | The plants that the rule reported, out of the plants made for it. |

The rules were narrowed after the readers' labels came in. For a narrowed rule, wrong advice
counts the labelled reports that the rule still makes and that are still wrong, so the number is
not a clean measurement. Recall was measured on the rules before that narrowing.

Recall on code in other projects is likely lower than the table says. Most plants were inserted
into a file in shapes the judges chose, and such shapes are closer to the documented examples than
the code in other projects is. For the metric and pipeline rules, recall was 190 of 280 on plants turned from a right
form already in the file, against 311 of 357 on inserted plants.

## Leakage

Each rule in the group reports a form that lets the test rows, or every row, shape what the
model learns, so the score on the test rows is too high. The score proofs use data whose target
is drawn apart from every feature, where an honest AUC is 0.5, and give the mean over 30 seeds.

A step that doesn't read the target moved the test AUC by at most 0.006 when it was fitted before
the split, so the rules leave it alone. Such steps are a scaler, an imputer, PCA, a binner, a
one-hot encoder and a text vectorizer.

- fit-on-test
- fit-before-split
- cross-val-on-fitted-data
- tune-on-test
- stateful-function-transformer

## Splits and groups

Each rule in the group reports a split of the rows that leaks or gives no score.

- groups-ignored
- missing-groups
- leave-one-out-score

## Metrics and scoring

Each rule in the group reports a metric given the wrong values, a score on the wrong rows, or a
scorer name that doesn't exist.

- auc-from-labels
- metric-arguments-swapped
- score-on-training-rows
- unknown-scoring-name

## Removed and deprecated forms

Each rule in the group reports a form that raises or warns on scikit-learn 1.9.1. Each form was
run under `python -W error`, where the old form raised and the new form ran clean. The messages
take each replacement from the 1.9.1 source or from the error text, such as the docstring that
says `sparse` was renamed `sparse_output` in 1.2.

A fix of a removed form turns code that raises into code that runs, so no such fix can be bit
equal or safe. A fix that drops a keyword deletes the comments inside the argument
list, and it joins a call written over several lines onto one line.

- removed-in-sklearn
- deprecated-in-sklearn
- removed-metric-api
- deprecated-metric-api
- fit-params-keyword
- column-transformer-removed

## Pipeline and metadata routing

Each rule in the group reports a form that raises at construction or at fit on scikit-learn
1.9.1. No held-back file uses metadata routing, a misnamed step, `TransformedTargetRegressor` in a
wrong mix, or pandas output on a sparse encoder. The rules in the group therefore made no report
there and have no number for wrong advice.

- metadata-routing
- pipeline-fit-params
- pipeline-step-form
- transformed-target-pair
- pandas-output-sparse

## Work done twice

Each rule in the group reports work that scikit-learn already did, and the new form does it once.
The numbers are speed proofs on scikit-learn 1.9.1, on one thread, as the median of 7 runs of each
form at two data sizes. A rule reports only the classes where both sizes passed 1.5 times.

- predict-per-row
- score-after-predict
- predict-and-predict-proba
- cross-val-score-per-metric
- vectorizer-fit-then-transform
- refit-per-n-estimators
- tsne-exact

## Reproducible results

Pass `random_state` wherever scikit-learn shuffles or draws, so two runs give the same split and the same score.

- random-state-without-shuffle
- missing-random-state
