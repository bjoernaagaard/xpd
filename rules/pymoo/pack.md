# pymoo

The pymoo rules find code from older pymoo releases that raises or does nothing on pymoo 0.6.2,
problem classes that hand pymoo wrong numbers, runs whose result is not the one the code asks
for, and reference points and weights that give another number than the code means. They run on
files that import [pymoo](https://pymoo.org/), a library for single-objective and
multi-objective optimization with evolutionary algorithms.

| Group | What its rules report |
| --- | --- |
| Older releases | imports, keywords and options of pymoo 0.4 and 0.5 that raise, warn or do nothing on 0.6.2 |
| The problem class | an `_evaluate` that hands pymoo wrong numbers or raises |
| Running an algorithm | `minimize` keywords that pymoo never reads, and an algorithm read or reused after a run |
| Random numbers | runs without a seed, and draws from `np.random` that the seed does not reach |
| Indicators and decisions | reference points and weights that give another number than the code means |

Every number on the pymoo rule pages was measured on pymoo 0.6.2 with NumPy 2.4.6, unless another
release is named. The examples assume that the pymoo names they use are imported, and names such
as `problem`, `F` and `pf` stand for values from your program.

xpd picks the rule documents that match the pymoo release in the project's venv. pymoo renamed
its modules in 0.5.0, and it removed `pymoo.factory` and most keywords of the 0.4 API in 0.6.0
and 0.6.1. It moved its random numbers to a generator of its own in 0.6.1.6. A rule document about
such a change carries `since`. A document carries `until` as well when its advice differs by
release, and a second document then gives the advice of the newer releases. xpd reads the
installed version from the venv as it does for pandas:

```text
on(check a file that imports pymoo)
  if the venv has no pymoo, or there is no venv
    run every document that has no until
  else
    run each document whose since is at most the installed version
      and whose until, if it has one, is above it
```

A project on pymoo 0.5.0 therefore gets no report about 0.6. The table gives the three moves that
need `until`, and a project gets the module of the release in its venv. From 0.6.0 to 0.6.1.5
the import from `pymoo.parallelization` raised ModuleNotFoundError.

| Names | 0.5.0 | 0.6.0 to 0.6.1.5 | 0.6.1.6 and later |
| --- | --- | --- | --- |
| `StarmapParallelization`, `DaskParallelization` | not in pymoo | `pymoo.core.problem` | `pymoo.parallelization` |
| Nelder and Mead, pattern search | `pymoo.algorithms.soo.nonconvex.nelder_mead`, `pattern_search` | `pymoo.algorithms.soo.nonconvex.nelder`, `pattern` | `pymoo.algorithms.soo.nonconvex.nelder`, `pattern` |
| `DES` | `pymoo.algorithms.soo.nonconvex.de` | not in pymoo | `pymoo.operators.selection.des` from 0.6.2 |

Every rule ties what it reports to pymoo by a name, because the rules match the shape of the code
and do not know the type of a variable. A class or
a function must be one that the file imports from pymoo, such as `Problem` from
`pymoo.core.problem` or `minimize` from `pymoo.optimize`. An object must be a name that the file
binds to such a class. One document of `deprecated-in-pymoo`, about the `"feasible"` key, reads
the name of a variable instead, and its page says so.

## The proof each rule needs

Every rule document has a proof that was run, and the table gives the four kinds of proof.

| Proof | How it was run | What it shows |
| --- | --- | --- |
| interpreter | The old form and the new form ran. For a removed import, the old form also ran on the last release that has it and on the next one, from the eleven releases from 0.4.2.2 to 0.6.2 on PyPI. | The old form raises or warns and the new form runs clean. `since` names the first release where the old form raises. |
| value | The old and the new form ran on the same input. | Both results. |
| rerun | The old form ran twice. | The two results differ, and the new form gives the same result twice. |
| speed | Both forms were timed. | The two times, and whether the two forms return the same values. |

A document is `error` only on an interpreter proof or a value proof, and its message quotes what
was measured. A document that can also match code that is meant, such as a mutation that few
individuals should get, is a `warning`.

The lists inside the rules were read from the installed wheels, the package files that PyPI
serves for each release, and not written by hand.

| List | Entries | Check |
| --- | --- | --- |
| removed modules | 130 | each import ran on the release before and on the release named |
| names that a kept module lost | 49 | the same |
| termination names that 0.6.0 dropped | 10 | read from the wheels |
| algorithm classes | 45 | read from the wheels |
| replacement modules that a fix writes | 89 | each was imported on 0.6.2 with every name the fix lets through |

The first two lists leave out `pymoo.experimental`, `pymoo.usage`, `pymoo.docs` and
`pymoo.vendor`.

The rules were then run over two sets of code, and the reports were read. A rule that reported
correct code was narrowed, and the table after the list of sets gives what is left.

| Set | Files | What the files are |
| --- | --- | --- |
| pymoo's own code at the 0.6.2 tag | 541 | the library, its examples, its tests and the Python code of its documentation pages |
| public code | 2,046 from 409 repositories | files that import pymoo, found by a code search on GitHub. Another 2,464 files in the repositories are copies of pymoo's own modules and are not counted. |

| Rule | Severity | Proof | Reports on pymoo's own code | Reports on public code | Wrong advice | Fix |
| --- | --- | --- | --- | --- | --- | --- |
| [`removed-in-pymoo`](removed-in-pymoo.md) | error | interpreter: ModuleNotFoundError, ImportError, Exception, AttributeError | 0 | 1,694 | 0 of 40 read | unsafe |
| [`deprecated-in-pymoo`](deprecated-in-pymoo.md) | warning | interpreter: DeprecationWarning, and the same class or array | 2 | 97 | 0 of 36 read | safe |
| [`n-constr-keyword`](n-constr-keyword.md) | info | value: the same X, F and G | 2 | 232 | 0 of 28 read | safe |
| [`positional-problem-bounds`](positional-problem-bounds.md) | error | value: `xl` [1, 1] and `xu` None, and interpreter: TypeError | 0 | 0 | no reports | none |
| [`ignored-problem-keyword`](ignored-problem-keyword.md) | error, warning | value: [0.66, 0.93, 1.26] against [0.14, 0.77, 1.94], and 0 calls of the starmap | 1 | 67 | 0 of 9 read | none |
| [`setup-positional-termination`](setup-positional-termination.md) | error | value: past 30 generations where 7 were asked | 0 | 1 | 0 | unsafe |
| [`has-terminated-assignment`](has-terminated-assignment.md) | error | value: 0 more generations against 15 | 0 | 4 | 0 | none |
| [`mutation-prob-per-individual`](mutation-prob-per-individual.md) | warning | value: 2 percent of the individuals against 57 percent | 0 | 48 | 0 of 14 read | unsafe |
| [`indicator-normalize-ignored`](indicator-normalize-ignored.md) | error | value: 25.25 against 0.082 | 0 | 0 | no reports | unsafe |
| [`objectives-stacked-by-row`](objectives-stacked-by-row.md) | error | value: a least f2 of 0.003 against 10 | 0 | 0 | no reports | unsafe |
| [`evaluate-returns-value`](evaluate-returns-value.md) | error | value: F is inf | 0 | 0 | no reports | none |
| [`constraint-not-declared`](constraint-not-declared.md) | error | interpreter: Exception | 0 | 0 | no reports | none |
| [`evaluate-indexing`](evaluate-indexing.md) | error | interpreter: IndexError, Exception | 0 | 24 | 0 | none |
| [`elementwise-runner`](elementwise-runner.md) | error | interpreter: TypeError, and value: 0 calls of the runner | 0 | 0 | no reports | none |
| [`minimize-ignored-keyword`](minimize-ignored-keyword.md) | error | value: 1,000 generations where 7 were asked, and rerun | 2 | 1 | 0 | unsafe |
| [`algorithm-read-after-minimize`](algorithm-read-after-minimize.md) | error | value: None, and interpreter: TypeError | 0 | 0 | no reports | none |
| [`reused-algorithm-without-copy`](reused-algorithm-without-copy.md) | error | value: the F of the first run | 0 | 0 | no reports | none |
| [`unread-history`](unread-history.md) | info | speed: 2.6 times, and 20.3 MB against 0.13 MB | 6 | 110 | 1 of 6 read | none |
| [`moead-arguments`](moead-arguments.md) | error | interpreter: TypeError, AttributeError | 0 | 3 | 0 | none |
| [`unseeded-run`](unseeded-run.md) | info | rerun: the result differs | 21 | 286 | 0 of 24 read | none |
| [`numpy-seed-ignored`](numpy-seed-ignored.md) | warning | rerun: the result differs | 23 | 148 | 0 of 32 read | none |
| [`hv-reference-point`](hv-reference-point.md) | warning, error | value: 0.09 against 0.37, and interpreter: AssertionError | 0 | 2 | 0 | none |
| [`decision-weights`](decision-weights.md) | warning | value: another solution is picked | 0 | 0 | no reports | none |

A value proof gives the old form's result against the new form's. Wrong advice counts the reports
where following the message would not fix a real problem in that code. Where a rule has more
reports than were read, the cell says how many were read, and every report of the other rules
was read.

The one report with wrong advice is on a documentation page of pymoo that saves the history for
a later page to plot, and xpd reads the later page as another file. The public reports of
`unread-history` were not judged, because 94 of the 110 are in a repository where another file
names `history`, and the rule is `info` for the same reason. Apart from the
one report with wrong advice, the 57 reports on pymoo's own code are real.
For example, its examples pass `n_gen=100` to `minimize`, where the keyword does nothing, and its
documentation reads the deprecated `"feasible"` key. The custom operators in its guide draw from
`np.random`.

The rules were narrowed in two rounds, the first after the first run and the second after a
review. The reports on pymoo's own code and on the public files were the same before and after
the second round, line for line.

| Rule | Right code it matched | Change |
| --- | --- | --- |
| `algorithm-read-after-minimize` | 29 callbacks and comprehensions with a variable named like the algorithm | needs the read in the scope of the run, and reads a walrus as a new binding |
| `evaluate-returns-value` | one base class that returns `NotImplemented` | leaves the class alone |
| `unread-history` | runs inside functions, whose caller may read the history | reads runs outside every function |
| `evaluate-indexing`, `objectives-stacked-by-row` | a class that can be built with `elementwise=True` | leave the class alone |
| `reused-algorithm-without-copy` | a loop over the algorithms | leaves the loop alone |
| `has-terminated-assignment`, the `calc` document of `removed-in-pymoo` | a name with no tie to pymoo | need a name that the file ties to pymoo |

The first three rows are the first round. The review also added `positional-problem-bounds`,
which made no report on either set.

Recall was not measured, because no set of real pymoo code was held back. The rule table says
how often a rule is wrong when it reports, and not how often it misses.

The pack has no rule for the habits in the table, because a rule cannot tell from the code of one
file whether the habit is a mistake. The last column gives the reason for each habit.

| Habit | What was measured | Why there is no rule |
| --- | --- | --- |
| A constraint written with the wrong sign | `out["G"]` must be at most 0 where the constraint holds | The direction a constraint is meant in is not in the code |
| A maximized objective that is not negated back | pymoo only minimizes, so `res.F` holds the negated value | The problem class and the code that reads `res.F` are mostly in different files |
| `pop_size` below the number of reference directions in `NSGA3` | pymoo prints a warning, and 54 solutions came back for 91 directions | The number of directions is a binomial of two arguments |
| A problem without `xl` and `xu` | `GA` raised TypeError when it sampled | A binary or a permutation problem needs no bounds |
| Variables given with `vars` and run with `NSGA2` | TypeError, `'>=' not supported between instances of 'dict' and 'dict'` | The problem and the algorithm are tied only by the call that runs them |
| A hypervolume of objectives on another scale than the reference point | `HV(ref_point=[1.1, 1.1])` gave 0.0 for a first objective between 100 and 900 | The scale of the objectives is in the data |
| `ASF` on objectives that are not normalized | It picked another solution than on normalized objectives | Whether the objectives share a scale is in the data |
| Silencing the warning about functions that are not compiled | With the compiled functions switched off, sorting 2,000 points took 27 times as long on pymoo 0.6.1.6 and no longer than before on 0.6.2 | Nothing is slower on the current release |

## Older releases

The rules in the group report forms that worked on an older pymoo and that raise, warn, or are
accepted and do nothing on pymoo 0.6.2.

- removed-in-pymoo
- deprecated-in-pymoo
- n-constr-keyword
- positional-problem-bounds
- ignored-problem-keyword
- setup-positional-termination
- has-terminated-assignment
- mutation-prob-per-individual
- indicator-normalize-ignored

## The problem class

The rules in the group report an `_evaluate` that hands pymoo wrong numbers or raises. The two
problem classes get `x` in different shapes, and both write their results to the dictionary
`out`.

| Class | What `_evaluate` gets as `x` | The first variable |
| --- | --- | --- |
| `Problem` | the whole population, as an array with one row per solution | `x[:, 0]` |
| `ElementwiseProblem` | one solution, as an array with one value per variable | `x[0]` |

- objectives-stacked-by-row
- evaluate-returns-value
- constraint-not-declared
- evaluate-indexing
- elementwise-runner

## Running an algorithm

The rules in the group report `minimize` keywords that pymoo never reads, and an algorithm that
is read or run again after `minimize` copied it.

- minimize-ignored-keyword
- algorithm-read-after-minimize
- reused-algorithm-without-copy
- unread-history
- moead-arguments

## Random numbers

The rules in the group report runs that give another result each time. Since pymoo 0.6.1.6 the
library draws from a generator that `minimize` builds from `seed`, and `np.random.seed` does not
reach it.

- unseeded-run
- numpy-seed-ignored

## Indicators and decisions

The rules in the group report a reference point or weights that give another number than the
code means.

- hv-reference-point
- decision-weights
