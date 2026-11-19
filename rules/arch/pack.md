# arch

The arch rules find code that raises on arch 9.0, options and arguments that arch refuses or
never reads, returns in units that break the fit, and random draws without a seed. They run on
files that import [arch](https://bashtage.github.io/arch/), a library for ARCH and GARCH
volatility models, unit root and cointegration tests, bootstraps and multiple comparison tests.

| Group | What its rules report |
| --- | --- |
| Removed API | API that arch 8.0 and 9.0 removed |
| Options and arguments | a string or a number that arch refuses, and an argument that arch accepts and never reads |
| The units of the data | returns as decimals, and results that are read in rescaled units |
| Forecasts and fits | forecasts that raise or that compute the same rows many times, and fits that print in a loop |
| Bootstraps, tests and covariance estimators | draws without a seed, results read before `compute`, and a bandwidth that changed its meaning in 9.0 |

Every number on the arch rule pages was measured on arch 9.0.0 with NumPy 2.5.5, pandas 3.1.1, SciPy
1.18.1 and statsmodels 0.15.0, unless another release is named. The examples assume that the
arch names they use are imported, and names such as `returns`, `prices`, `losses` and `x` stand
for values from your program.

xpd skips a rule document about a newer arch than the one in the project's venv. arch 8.0
removed the `RandomState` paths of the bootstraps and the distributions. arch 9.0 removed
`arch.__future__` and the `type` keyword of `hedgehog_plot`, and it made some attributes read
only. It also started to check every argument. A rule document about such a change carries
`since`, and xpd reads the installed version from the venv as it does for pandas.

```text
on(check a file that imports arch)
  if the venv has no arch, or there is no venv
    run every document
  else if the document's since is above the installed version
    skip the document           # a project on arch 7.2 gets no report about what 8.0 removed
  else
    run the document
```

Each rule ties its report to a name that the file binds to arch, because the rules match the
shape of the code and do not know the type of a variable. `samples/arch/names.py` shows each
case.

| Code | Read by the rules |
| --- | --- |
| a call by the name of an arch class or function, such as `arch_model(...)` or `IIDBootstrap(...)` | yes |
| a short name that another library may have, such as `SPA`, `MCS` or `Normal` | only when the file imports it from arch |
| a method such as `fit`, `forecast` or `conf_int` on a variable | only when the file assigns the variable from an arch name and from nothing else |
| the same variable bound as a parameter, a loop variable, an `as` target or one name of a tuple assignment | no |
| an option or a value written in the call, such as `vol="gjr"` | yes |
| a variable in place of the option or the value | no |

## The proof each rule needs

Every rule document has a proof that was run, and the table gives the five kinds of proof.

| Proof | How it was run | What it shows |
| --- | --- | --- |
| interpreter | The old form ran on arch 9.0.0. A document with `since` also ran on the release before the one it names. | The old form raises and the new form runs. On the release before, the old form ran or only warned. |
| same result | A fit or a test ran with and without the argument. | Both gave the same parameters or the same statistic, so arch does not read the argument. |
| numbers | Both forms ran on the daily S&P 500 returns that ship with arch. | The two numbers. |
| speed | Both forms were timed as the median of 3 runs. | The two times, and whether the two forms return the same values. |
| rerun | The old form ran twice in fresh objects. | The two results differ, and the new form gives the same result twice. |

A document is `error` only on an interpreter proof, and its message names the exception. A
document that can also match right code, such as `rescale=True` on data that needs no
rescaling, is a `warning`.

The rules were then run over two sets of real code, and a rule that reported correct code was
narrowed or dropped.

| Set | Files | What the files are |
| --- | --- | --- |
| arch's own code at the 9.0.0 tag | 202 | 69 library files, 104 test files, the code blocks of 16 documentation pages, 9 notebooks and 4 scripts |
| public code | 1,303 from 861 repositories | 795 Python files and 508 notebooks that GitHub code search found for the import lines of arch, of which 1,268 import arch |

Every report was read except the 291 reports of `arch-unseeded-draws`, which a script checked
by looking in each file for a later `seed` call or `state` assignment on the same variable. The
table gives the reports that were left after the rules were narrowed.

| Rule | Severity | Proof | Reports on arch's own code | Reports on public code | Wrong advice | Fix |
| --- | --- | --- | --- | --- | --- | --- |
| [`removed-in-arch`](removed-in-arch.md) | error | interpreter: ModuleNotFoundError, ImportError, TypeError, AttributeError, ValueError | 1 | 34 | 0 | unsafe |
| [`arch-unknown-option`](arch-unknown-option.md) | error | interpreter: ValueError | 0 | 13 | 0 | unsafe |
| [`arch-ignored-argument`](arch-ignored-argument.md) | warning | same result, and rerun for the two seeds | 10 | 21 | 1 | none |
| [`arch-rejected-number`](arch-rejected-number.md) | error | interpreter: TypeError, ValueError | 1, over 189 files | not run | 1 | unsafe |
| [`unscaled-returns`](unscaled-returns.md) | warning | numbers: mu 4.02 against 0.056, with optimizer code 4 | 0 | 21 | 0 | none |
| [`rescaled-units`](rescaled-units.md) | warning | numbers: a variance forecast 10,000 times the data's | 4 | 14 | 7 | none |
| [`analytic-forecast-horizon`](analytic-forecast-horizon.md) | error | interpreter: ValueError | 0 | 0 | no reports | none |
| [`forecast-per-origin`](forecast-per-origin.md) | warning | speed: 0.106 seconds against 0.0005 | 1 | 0 | 0 | none |
| [`reindex-is-default`](reindex-is-default.md) | info | same result: equal frames with and without the argument | 20 | 61 | 0 | safe |
| [`fit-output-in-loop`](fit-output-in-loop.md) | info | numbers: 240 lines for 20 fits against 0 | 0 | 15 | 0 | unsafe |
| [`arch-unseeded-draws`](arch-unseeded-draws.md) | info | rerun: the result differs | 109 | 182 | 1 | none |
| [`results-before-compute`](results-before-compute.md) | error | interpreter: RuntimeError | 2 | 0 | 2 | none |
| [`opt-bandwidth-as-bandwidth`](opt-bandwidth-as-bandwidth.md) | warning | numbers: 0.5245 against 0.5158 | 0 | 0 | no reports | unsafe |

Wrong advice counts the reports where following the message would not fix a real problem in that
code, and each rule's page says which reports they are. Before the rules were narrowed, the
first run made 508 reports.

| Code | Reports | Wrong advice |
| --- | --- | --- |
| arch's library source | 1 | 1, a placeholder bootstrap that a base class builds and never draws from |
| arch's tests, notebooks and documentation | 146 | 7 |
| public code | 361 | 3 |

`arch-rejected-number` was added after a review, when the public code was no longer on disk. It
ran over the 189 Python files of arch's repository and made one report, in a test that checks
the error it reports.

A rule that reported `forecast(start=...)` on a result whose `fit` had no `last_obs` was
written and dropped. The parameters of such a fit have seen the rows that are forecast, and the
forecasts did change, but most of the reports were on right code.

| Measurement on 1,500 S&P 500 returns, forecasts from row 1,000 on | Value |
| --- | --- |
| mean difference from the forecasts of a fit that stopped at row 1,000 | 13% |
| QLIKE loss with the fit on all rows | 0.142 |
| QLIKE loss with the fit that stopped at row 1,000 | 0.181 |
| reports on public code | 14, of which 7 start at the last row and forecast the future, and 4 start at row 0 to draw the fitted path |
| reports on arch's own code | 11, all examples and tests that forecast the fitted rows on purpose |

Recall was not measured, because no set of real arch code was held back. The table says how
often a rule is wrong when it reports, and not how often it misses. A rule that follows a
variable is silent when the file binds the same name to something else anywhere, also in
another function. For example, a file that uses `res` for an arch result and for another object
gets no report on `res`.

## Removed API

`removed-in-arch` reports code that raises on arch 9.0.0. Each form was run on the release
before the removal and on the release that made it, both installed from PyPI, because the
release notes of 8.0 do not list what 8.0 removed. No fix of a removed form is safe, because
the old form raises and its result cannot be compared with the result of the new form.

- removed-in-arch

## Options and arguments

The rules in the group report an argument that arch 9.0 refuses or never reads. arch 9.0 checks
every argument when a function is called, and it still ignores an argument that does not apply
to the model that was asked for.

| Rule | What it reports | What happens |
| --- | --- | --- |
| `arch-unknown-option` | a string that is not one of the option's values | ValueError |
| `arch-ignored-argument` | an argument that arch accepts and never reads | the code runs and estimates another model or test than the one it describes |
| `arch-rejected-number` | a number that arch refuses | TypeError or ValueError |

- arch-unknown-option
- arch-ignored-argument
- arch-rejected-number

## The units of the data

Give arch returns in percent, and divide a result by `scale` when you let arch rescale. arch
estimates by maximum likelihood, and the optimizer works badly when the variance of the data is
far from 1. arch warns `DataScaleWarning` when the variance is outside the range it accepts, and
its message asks for a value between 1 and 1000. Daily returns as decimals have a variance near
0.0001, and the 5,030 daily S&P 500 returns in `arch.data.sp500` have 0.000145.

- unscaled-returns
- rescaled-units

## Forecasts and fits

The rules in the group report forecasts that raise or repeat work, and fits that print. The
table gives the defaults of the two calls.

| Call | Default | Effect |
| --- | --- | --- |
| `forecast` | `method="analytic"`, `horizon=1`, `reindex=False` | forecasts from the last row unless `start` names an earlier one |
| `fit` | `disp="final"`, `update_freq=1` | prints the steps of the optimizer unless `disp="off"` |

- analytic-forecast-horizon
- forecast-per-origin
- reindex-is-default
- fit-output-in-loop

## Bootstraps, tests and covariance estimators

The rules in the group report random draws without a seed, test results read before
`compute`, and a bandwidth whose meaning changed in arch 9.0.

- arch-unseeded-draws
- results-before-compute
- opt-bandwidth-as-bandwidth

## What the pack leaves out

The pack has no rule for the habits in the table, and the last column gives the reason for each.

| Habit | What was measured | Why there is no rule |
| --- | --- | --- |
| `SPA`, `StepM` and `RealityCheck` studentize the losses since arch 9.0 when `studentize=True`, the default, which did nothing before | On the same seeded losses the p-values of `SPA` were 0.6733, 0.91 and 0.96 on 8.0.0 and 0.67, 0.8967 and 0.9533 on 9.0.0, and the lowest critical value moved from 0.1353 to 1.5984. `studentize=False` on 9.0.0 gave the numbers of 8.0.0. | A rule would report nearly every call, because few calls say `studentize` |
| `nested=True` on the same tests runs a bootstrap inside every replication since 9.0 | With 300 replications `SPA` took 1.14 seconds, against 0.0105 without it and 0.0109 on 8.0.0 | The option asks for the extra work |
| `forecast(start=...)` on a fit without `last_obs` forecasts rows that the fit has seen | The numbers are under "The proof each rule needs" | The rule reported more right code than wrong and was dropped |
| `starting_values` from the last window of a rolling fit | 40 windows took 0.209 seconds without them and 0.200 with them | The fit was not faster |
| an `IIDBootstrap` on a series that is correlated in time, where arch's documentation gives the block bootstraps and `optimal_block_length` | `CircularBlockBootstrap` and `MovingBlockBootstrap` raise TypeError since 9.0 on a block size that is not a whole number, such as the 55.04 that `optimal_block_length` returned for one series. `StationaryBootstrap` takes it. | A pattern cannot see whether a series is correlated, and the block size is a variable in such code |
| a unit root test on the wrong series, or a p-value read against the wrong null hypothesis | On 1,000 log prices of the S&P 500, `ADF` gave a p-value of 0.648 and `KPSS` gave 0.0001, because the null of `ADF` is a unit root and the null of `KPSS` is no unit root | A pattern sees neither the series nor the reading |
| a `block_size` given as a float | It was a variable in every file that was read | `arch-rejected-number` reads only a number written in the call |
