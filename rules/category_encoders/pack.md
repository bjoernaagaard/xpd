# Category Encoders

The Category Encoders rules report code that leaks the target into the test rows, arguments that
give wrong codes without an error, code that raises on category_encoders 2.11.2, and forms that a
release removed or deprecated, in each file that imports `category_encoders`. Every number about the
Category Encoders rules was measured on category_encoders 2.11.2 with scikit-learn 1.9.1, pandas 3.1.1 and NumPy
2.5.5 on Python 3.13. The examples assume `import category_encoders as ce` and that the
scikit-learn names they use are imported. Names such as `X_train`, `y_test` and `encoder` stand for
values from your program.

`OneHotEncoder`, `OrdinalEncoder` and `TargetEncoder` exist in scikit-learn as well. A rule about
one of the three reports a call only with proof that the class comes from category_encoders. The
proof is the module in front of the name, as in `ce.OneHotEncoder`, or an import of the name from
`category_encoders` in the same file. A file that imports the same name from both libraries gives
no proof, so the rules about the three classes stay silent there. The other 20 encoder names
belong to category_encoders alone and need no proof.

A rule document whose advice needs a newer release carries `since`. xpd reads the installed
version from the project's venv and skips the document on an older release. When the venv has no
category_encoders, or there is no venv, every document runs.

The scikit-learn pack already reports a target encoder that is fitted before the split, fitted
before cross-validation, or refitted on the test rows, through
[`fit-before-split`](../sklearn/fit-before-split.md),
[`cross-val-on-fitted-data`](../sklearn/cross-val-on-fitted-data.md) and [`fit-on-test`](../sklearn/fit-on-test.md). A file
that uses category_encoders with a split nearly always imports scikit-learn, so the Category
Encoders pack doesn't repeat the three rules. It adds the one form they leave alone, in
[`encode-before-split`](encode-before-split.md).

## The proof each rule needs

Each `error` and `warning` document ships only with a proof that was run, and the message quotes
what the proof measured. A proof is one of the following kinds:

| Proof | What runs | When it passes |
| --- | --- | --- |
| score | The old and the new form, over 30 seeds on data whose target is drawn apart from every feature, where the honest AUC is 0.5. | The old form's mean is more than three standard errors above 0.5, and the new form's mean is within 0.02 of it. |
| result | The old form, with the codes it gives printed next to the codes of the new form. | The old form gives a code that is wrong for every input of that shape, such as -1.0 for every row. |
| interpreter | The old form under `python -W error`. | The old form raises an exception or a warning, and the new form runs clean. |
| speed | Both forms on one thread, as the median of 7 runs, at 10,000 and 100,000 rows. | The new form is at least 1.5 times faster at both sizes. |

category_encoders 2.11.2 itself sets a pandas option that pandas 3.1 deprecates, so the
interpreter proofs ignored the pandas warning about the option.

A document is `info` when no proof passed for it, or when its advice depends on something the code
doesn't show, such as the dtype of a column. `sklearn-ships-encoder` passed its speed proof and is
still `info`, because the two libraries return different things and the swap is a choice.
`nan-into-pipeline` and `ordinal-mapping-unknown` are `warning`, because their proofs show a raise
and a wrong code for a category that the training rows lacked, and the code doesn't show whether
new data brings one.

The rules were run over two sets of code:

- the library's own code at its latest release, which is the 86 Python files of the repository
  and 35 more blocks taken from its docstrings, its `.rst` pages and its README
- 55 files from public repositories that import `category_encoders`

After the runs, the rules below were narrowed, and the table gives the reports that are left:

- `invalid-handle-option` reported three scikit-learn `OneHotEncoder(handle_unknown="ignore")`
  calls in files that import the name from both libraries. A second import now removes the proof.
- `hashing-default-components` reported `HashingEncoder().hashing_trick(df, N=3)`, which passes
  the number of columns itself.
- `zero-regularization` reported `CountTargetEncoder(smoothing=0)` inside `assertRaises`. Every
  rule about a form that raises now leaves alone code that expects the error.
- `sklearn-ships-encoder` reported 6 encoders handed to `PolynomialWrapper` or `NestedCVWrapper`,
  which take only the encoders of category_encoders.

| Rule | Severity | Proof | Reports on the library | Reports on 55 public files | Fix |
| --- | --- | --- | --- | --- | --- |
| [`transform-with-test-target`](transform-with-test-target.md) | error, info | score: 0.691 against 0.517 AUC | 4 | 0 | unsafe |
| [`encode-before-split`](encode-before-split.md) | error | score: 0.655 against 0.495 AUC | 0 | 0 | none |
| [`ordinal-mapping-pairs`](ordinal-mapping-pairs.md) | error | result: every code is -1.0 | 0 | 0 | unsafe |
| [`ordinal-mapping-unknown`](ordinal-mapping-unknown.md) | warning | result: -1.0 for a category the mapping lacks | 3 | 1 | none |
| [`zero-regularization`](zero-regularization.md) | error | result: inf. Interpreter: ValueError | 0 | 0 | none |
| [`hashing-default-components`](hashing-default-components.md) | warning | result: 100 values give 8 distinct rows | 10 | 1 | none |
| [`cols-left-to-dtype`](cols-left-to-dtype.md) | info | result: 1 column where 5 were meant | 1 | 0 | none |
| [`nan-into-pipeline`](nan-into-pipeline.md) | warning | interpreter: ValueError at predict | 0 | 0 | none |
| [`supervised-encoder-without-target`](supervised-encoder-without-target.md) | error | interpreter: TypeError, ValueError | 0 | 0 | none |
| [`invalid-handle-option`](invalid-handle-option.md) | error | interpreter: ValueError, TypeError | 0 | 0 | none |
| [`named-cols-after-array-step`](named-cols-after-array-step.md) | error | interpreter: ValueError | 0 | 0 | none |
| [`wrapper-fit-returns-none`](wrapper-fit-returns-none.md) | error | interpreter: AttributeError | 0 | 0 | none |
| [`removed-in-category-encoders`](removed-in-category-encoders.md) | error | interpreter: TypeError, AttributeError, KeyError | 0 | 0 | unsafe |
| [`deprecated-in-category-encoders`](deprecated-in-category-encoders.md) | warning, info | interpreter: FutureWarning. Result: equal frames | 0 | 0 | safe |
| [`sklearn-ships-encoder`](sklearn-ships-encoder.md) | info | speed: 1.67 times at the lowest | 61 | 16 | none |
| [`encoder-noise-without-seed`](encoder-noise-without-seed.md) | info | result: two runs differ by up to 0.15 | 4 | 0 | none |

A score proof gives the old form's mean against the new form's mean. The speed proof gives the
lowest ratio over the three classes and both sizes.

None of the 83 reports on the library names a line where the rule's claim is false. The advice
still doesn't apply to most of them, because the library uses or tests its own classes:

- All 61 reports of `sklearn-ships-encoder` are the library's tests, examples and documentation
  building its own `OneHotEncoder`, `OrdinalEncoder` or `TargetEncoder`.
- The 4 reports of `transform-with-test-target` are tests that pass a target to `transform` on
  purpose, to check what the encoder does with it.
- The 10 reports of `hashing-default-components` are calls that do run with 8 columns, of which 4
  are in tests, 4 in benchmark examples and 2 in the documentation.
- The report of `cols-left-to-dtype` is an example that hands a `TargetEncoder()` two text
  columns, which it encodes.

The 18 reports on the 55 public files all name a category_encoders class. Recall was not
measured. Most rules have no report on the public files, so nothing here says how often the old
forms occur.

## Leakage

Each rule in the group reports a target encoder that sees the labels of the test rows, so the
score on the test rows is too high. A target encoder codes a category by the labels of its rows,
so the code of a test row then holds part of its own label. The score proofs fit a random forest of 50 trees on
600 rows with one text column of 100 levels and a target drawn apart from it, and give the mean
test AUC over 30 seeds.

- transform-with-test-target
- encode-before-split

## Codes that are wrong without an error

Each rule in the group reports a form that runs and gives another number than the code asks for.

- ordinal-mapping-pairs
- ordinal-mapping-unknown
- zero-regularization
- hashing-default-components
- cols-left-to-dtype
- nan-into-pipeline

## Code that raises

Each rule in the group reports a form that raises on category_encoders 2.11.2. Code inside
`assertRaises`, `pytest.raises` or a `try` block with an `except` clause expects the error and is
left alone.

- supervised-encoder-without-target
- invalid-handle-option
- named-cols-after-array-step
- wrapper-fit-returns-none

## Removed and deprecated forms

Each rule in the group reports a form that a release of category_encoders removed or deprecated.
The release of each removal was found by reading the constructor of every class in the wheels of
1.3.0, 2.2.2, 2.5.1, 2.6.4, 2.7.0, 2.8.0, 2.8.1, 2.9.0, 2.10.0 and 2.11.2, and each exception was
raised on 2.11.2. Code inside `assertRaises`, `pytest.raises` or a `try` block with an `except`
clause expects the error or handles both releases, and is left alone.

- removed-in-category-encoders
- deprecated-in-category-encoders

## Encoders that scikit-learn ships

Use scikit-learn's own encoder where it ships the same one, because its `OneHotEncoder` ran `fit_transform` 2.7 to 4.5 times faster.

- sklearn-ships-encoder

## Reproducible results

Pass `random_state` to an encoder that adds noise, so two runs give the same codes.

- encoder-noise-without-seed

## What the pack leaves out

The pack has no rule for the following habits, which were looked at:

- Habits that a pattern can't see in the code:
  - A target of more than two classes given to `TargetEncoder` as whole numbers is coded as the
    mean of the class numbers, which means nothing. `PolynomialWrapper` is the answer, but a
    pattern cannot see how many classes a target has. `WOEEncoder` raises `ValueError: The
    target column y must be binary` in the same case.
  - `X` and `y` with indexes that do not match raise `ValueError` at fit, and a pattern cannot
    see an index.
  - `GLMMEncoder` needs statsmodels, which category_encoders stopped installing in 2.11, and
    raises `ImportError` without it. A package check runs for every project that has
    category_encoders and cannot ask whether a file uses that class, so the pack has none.
- Advice that the measurements didn't support:
  - `encoder.fit(X_train, y_train)` followed by `encoder.transform(X_train)` gives the training
    rows of a `LeaveOneOutEncoder` the plain mean of their category. On 600 rows whose 100
    levels each carry their own share of positives, with a random forest over 30 seeds, the form
    scored 0.637 AUC on the test rows against 0.580 with the target passed. The form the
    documentation asks for was the worse one here, so no rule reports the other.
  - On the same data `LeaveOneOutEncoder()` without `sigma` scored 0.580 against 0.559 with
    `sigma=0.05` in a forest, and both scored 0.636 in a logistic regression, so there is no
    proof that the noise helps.
  - With the default number of threads, one encoder per column in a loop was 1.15 to 1.25 times
    slower than one encoder with `cols` for `OrdinalEncoder` and `TargetEncoder`, and 3.5 to 5.4
    times slower for `OneHotEncoder`.
