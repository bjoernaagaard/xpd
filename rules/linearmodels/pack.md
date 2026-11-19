# linearmodels

The linearmodels rules report formulas and arguments that fit another model than the code reads
as, covariance options that give other standard errors than the code asks for, code that raises on
linearmodels 7.1, statsmodels names read on linearmodels results, and forms that a release removed
or deprecated. They run in each file that imports `linearmodels`.
[linearmodels](https://bashtage.github.io/linearmodels/) fits panel models, instrument models,
systems of equations and factor models of asset prices.

Every number about the linearmodels rules was measured on linearmodels 7.1 with pandas 3.1.1,
NumPy 2.5.5, SciPy 1.18.1, statsmodels 0.15.0 and formulaic 1.2.2 on Python 3.13, and a number for another
release names it. The data sets `wage_panel`, `mroz` and `french` are the ones in
`linearmodels.datasets`. The examples assume that the linearmodels names they use are imported and
that `sm` is `statsmodels.api`. Names such as `data`, `y`, `X` and `instruments` stand for values
from your program.

Most of the pack reports code written from memory of statsmodels, because linearmodels looks like
statsmodels and differs from it in the following small ways:

| What | statsmodels | linearmodels |
| --- | --- | --- |
| intercept in a formula | added | not added |
| covariance names | `"nonrobust"`, `"HC1"`, `"cluster"` | `"unadjusted"`, `"robust"`, `"clustered"` |
| result names | `bse`, `tvalues`, `resid` | `std_errors`, `tstats`, `resids` |
| `IV2SLS` arguments | `(endog, exog, instrument=None)`, in `statsmodels.sandbox.regression.gmm` | `(dependent, exog, endog, instruments)` |
| `summary` | a method | a property |

`OLS`, `IV2SLS`, `IVGMM` and `SUR` exist in statsmodels as well. A rule about one of the four
reports a call only with proof that the class comes from linearmodels. The proof is the module in
front of the name, as in `linearmodels.IV2SLS` or `lm.IV2SLS` after `import linearmodels as lm`, or
an import of the name from `linearmodels` in the same file. A file that imports the same name from
both libraries gives no proof, so the rules about the four classes stay silent there. The other 15
model classes of `linearmodels.__all__` have names of their own and need no proof.

A rule about `fit` or about a result reads only an object it can follow to a linearmodels class.
Such an object is a call on the class itself, such as `PanelOLS(y, X).fit(...)`, or a name that
its function, or the module outside any function, assigns from such calls and from nothing else.
The rule leaves alone a name that is also assigned from anything else, that a loop, a `with`
statement or a lambda parameter binds, or that comes in as an argument. A statsmodels model in the
same file is therefore not reported. The 950 public files hold 585 `cov_type=` strings with a
statsmodels name, such as `"HC3"` and `"cluster"`, and no rule reports one of them.

A rule reads an argument only when it is written out. A `cov_type`, a formula or a flag that is a
name, an f-string or a value unpacked from a dictionary is left alone.

A rule document whose advice needs a newer release carries `since`. xpd reads the installed version
from the project's venv and skips the document on an older release. When the venv has no
linearmodels, or there is no venv, every document runs. Each `since` was checked by installing
from PyPI the release before it and the release at it, and running the form on both:

| `since` | Releases run |
| --- | --- |
| 4.15 | 4.14 and 4.15 |
| 4.18 | 4.17 and 4.18 |
| 5.0 | 4.31 and 5.0 |
| 7.1 | 7.0 and 7.1 |

## The proof each rule needs

Each `error` and `warning` document ships only with a proof that was run, and the message quotes
what the proof measured. A proof is one of the following kinds:

| Proof | What runs | When it passes |
| --- | --- | --- |
| result | The old and the new form on the same data, with what each returns printed. | The old form gives another number than the code reads as, with no error and no warning, such as a coefficient of 0.2184 where 0.0338 is the one with an intercept. |
| interpreter | The old form under `python -W error`. | The old form raises an exception or a warning, and the new form runs clean. Old releases warn about their own dependencies, so on the old releases only a warning from linearmodels counted. |
| simulation | At least 1,000 draws of data whose true coefficient is known. | It counts how often the 95% interval covers the coefficient, or how often a true null is rejected at 5%. |
| speed | Both forms on one thread, as the median of 5 fits, at two sizes. | The new form is at least 1.5 times faster at both. |

A document is `info` when its advice depends on something the code doesn't show, such as the dtype
of a column or how the errors are related. A form that was run on 7.1 and on 7.0 gave the same
result on both unless the text says otherwise.

The rules were run over two sets of code:

- the library's own code at 7.1, 258 files in all:
  - the 229 Python files of the repository, of which 165 are tests
  - the code cells of its 12 notebooks
  - the code blocks of 9 `.rst` and `.md` pages
  - the doctest lines of 8 modules
- 950 files from 623 public repositories that mention `linearmodels`, found through the GitHub
  code search, at most 4 from one repository, with notebooks turned into their code cells

After the runs, the rules below were narrowed, and the table gives the reports that are left:

- `time-before-entity-index` reported 2 frames that no panel model reads, and one frame indexed by
  date and hour on purpose. The frame must now reach a panel model, and `hour` is a time.
- `statsmodels-result-name` reported `res.tvalues` behind `hasattr(res, "tstats")`. Every rule
  about a form that raises now leaves code under a `hasattr` check alone.
- `hand-written-hausman` reported a test of OLS against instruments, which the method of 7.1
  doesn't do. Its message now names `wu_hausman()` as well.
- `absorb-not-categorical` matched its own name in a comment. It now looks for `category`.

| Rule | Severity | Proof | Reports on the library | Reports on 950 public files | Fix |
| --- | --- | --- | --- | --- | --- |
| [`formula-without-constant`](formula-without-constant.md) | warning | result: 0.2184 against 0.0338 | 3 | 13 | none |
| [`effects-outside-panel-ols`](effects-outside-panel-ols.md) | error | result: equal coefficients with and without the term | 0 | 0 | none |
| [`effects-keyword-with-formula`](effects-keyword-with-formula.md) | error | interpreter: TypeError | 0 | 3 | none |
| [`formula-power-term`](formula-power-term.md) | error | result: 2 columns where 3 were written | 1 | 1 | none |
| [`first-difference-constant`](first-difference-constant.md) | error | interpreter: ValueError | 0 | 0 | none |
| [`weights-column-name`](weights-column-name.md) | error | interpreter: TypeError | 0 | 0 | none |
| [`panel-ols-without-effects`](panel-ols-without-effects.md) | warning | result: 0.0349 against 0.0698 | 42 | 37 | none |
| [`too-many-effects`](too-many-effects.md) | error | interpreter: ValueError | 0 | 0 | none |
| [`time-before-entity-index`](time-before-entity-index.md) | warning | result: 0.1622 against 0.2483 | 1 | 1 | unsafe |
| [`use-lsdv`](use-lsdv.md) | warning | speed: 17 times at the lowest | 2 | 2 | none |
| [`clustered-without-clusters`](clustered-without-clusters.md) | error | result: 0.0413 against 0.0494. Interpreter: TypeError | 4 | 31 | none |
| [`cov-option-without-cov-type`](cov-option-without-cov-type.md) | error | interpreter: ValueError, TypeError. Result: 0.2037 against 0.3002 | 0 | 0 | none |
| [`unknown-cov-type`](unknown-cov-type.md) | error | interpreter: UnknownOptionError | 2 | 0 | none |
| [`kernel-default-bandwidth`](kernel-default-bandwidth.md) | warning | result: 0.8022 on 7.0 against 0.3794 on 7.1 | 22 | 1 | none |
| [`panel-default-covariance`](panel-default-covariance.md) | info | simulation: 17.3% against 5.7% rejections | 325 | 372 | none |
| [`iv-missing-arguments`](iv-missing-arguments.md) | error | interpreter: TypeError | 0 | 2 | none |
| [`iv-arguments-swapped`](iv-arguments-swapped.md) | warning | result: 0.2021 against 0.0664 | 0 | 0 | none |
| [`iv-without-exog`](iv-without-exog.md) | warning | simulation: 2.002 against 0.990 for a true 1.0 | 4 | 2 | none |
| [`manual-two-stage`](manual-two-stage.md) | warning | simulation: 99.6% against 95.7% coverage | 0 | 0 | none |
| [`absorb-not-categorical`](absorb-not-categorical.md) | info | result: 3.397 against 1.022 for a true 1.0 | 0 | 5 | none |
| [`statsmodels-result-name`](statsmodels-result-name.md) | error | interpreter: TypeError, AttributeError | 0 | 15 | unsafe |
| [`invalid-compare-precision`](invalid-compare-precision.md) | error | interpreter: ValueError | 0 | 0 | none |
| [`hand-written-hausman`](hand-written-hausman.md) | info | none: the method runs on 7.1 and is absent on 7.0 | 4 | 25 | none |
| [`removed-in-linearmodels`](removed-in-linearmodels.md) | error | interpreter: ModuleNotFoundError, AttributeError | 0 | 0 | unsafe |
| [`deprecated-in-linearmodels`](deprecated-in-linearmodels.md) | warning | interpreter: FutureWarning. Result: equal order | 12 | 0 | safe |

A result proof gives the old form's number against the new form's number.

Of the 422 reports on the library, 399 are in its tests, which call the forms on purpose. They are
308 fits with the default covariance, 42 `PanelOLS` models without effects that are compared with
`PooledOLS`, 22 kernel estimators with the default bandwidth, and 11 uses of the deprecated names
inside the tests of the deprecations. One report names a line where the advice is wrong. The line is on a
documentation page that shows `set_index(['year','firm'])` for the `PanelOLS` that pandas had in
2017, which took the time first. One report is a defect of the library's own README, whose first
example writes `exper + exper ** 2` and fits a model without the squared term.

Of the 510 reports on the 950 public files, 108 are errors or warnings, and each was read. None names
a line where the rule's claim is false. The other 402 are `info`. Of them, 20 of the 372
`panel-default-covariance` reports, 14 `hand-written-hausman` reports and all 5
`absorb-not-categorical` reports were read, and each is a fit or a model of the kind the rule
describes. `unknown-cov-type` reports none of the 585 `cov_type=` strings with a statsmodels name,
and the 20 of them that were read are calls on statsmodels models.

Recall was not measured, so nothing here says how often a rule misses the form it describes.

## Formulas

Each rule in the group reports a formula that fits another model than it reads as. linearmodels
parses formulas with formulaic, and every model has a `from_formula`. The rules read a formula only
when it is a string in the `from_formula` call, as the first argument, the `formula` keyword, or a
value of the dictionary that a system model takes.

- formula-without-constant
- effects-outside-panel-ols
- effects-keyword-with-formula
- formula-power-term
- first-difference-constant
- weights-column-name

## Panel models

Each rule in the group reports a panel model that reads its index or its effects in another way
than the code means. A panel model takes a frame whose index has two levels, the entity first and
the time second. `PanelOLS` is the only class with effects, and it has none until it is told to.

- panel-ols-without-effects
- too-many-effects
- time-before-entity-index
- use-lsdv

## Covariance

Each rule in the group reports a covariance choice that gives other standard errors than the code
asks for, or that raises. `fit` takes `cov_type` and, next to it as keywords, the options of the
chosen covariance. Each model has its own short list of names, and none of them is a statsmodels
name.

- clustered-without-clusters
- cov-option-without-cov-type
- unknown-cov-type
- kernel-default-bandwidth
- panel-default-covariance

## Instruments and absorbed effects

Each rule in the group reports an instrument model or an absorbed effect that is set up in another
way than the code means. `IV2SLS`, `IVLIML`, `IVGMM`, `IVGMMCUE` and `IVJIVE` take four arrays in
the order dependent, exog, endog, instruments. None of the four has a default, and none of the
models adds a constant.

- iv-missing-arguments
- iv-arguments-swapped
- iv-without-exog
- manual-two-stage
- absorb-not-categorical

## Results

Read a linearmodels result by its own attribute names, pass `compare` a precision that it takes, and use the built-in Hausman test in place of one written by hand.

- statsmodels-result-name
- invalid-compare-precision
- hand-written-hausman

## Removed and deprecated forms

Each rule in the group reports a form that a release removed or deprecated. Each form was run on
the release before and the release at the version the rule names, both installed from PyPI.

- removed-in-linearmodels
- deprecated-in-linearmodels

## What the pack leaves out

The pack has no rule for the following forms:

- Forms that a pattern can't tell apart from right code:
  - Inputs whose labels don't match, which raise `ValueError` on 7.1 where 7.0 paired them by
    position. A pattern can't see an index.
  - A frame without a two level index, a time level of text, and regressors that the effects
    absorb. Each raises with a clear message, and none shows in the code.
  - The changes of 7.1 to weighted models, to `FamaMacBeth` p-values and to the Hausman and
    specification tests. The code is the same on both releases, so a rule would report every use.
- Forms that were tried and gave no reason for a rule:
  - `check_rank=False`, because on columns that are exactly collinear 7.1 raised
    `LinAlgError: Singular matrix`. Where it was tried, the flag didn't give wrong numbers in
    silence.
  - Patsy functions that formulaic lacks. `te()` raised, and `cr()` and `cc()` raised a rank error
    next to an intercept. `Q()`, `center()`, `standardize()`, `bs()` and
    `C(x, Treatment(reference=...))` all ran. No use of the three was found in the public files.
  - xxhash, which `AbsorbingLS` uses for its cache when it is installed, as a package check. A fit
    with two absorbed columns took 0.134 s with it and 0.134 s without on 100,000 rows, and
    1.130 s against 1.123 s on 1,000,000, with the default number of threads.
- `SUR`, `IV3SLS`, `IVSystemGMM` and the factor models, which have no rule of their own. The
  formula, covariance and result rules read them.
