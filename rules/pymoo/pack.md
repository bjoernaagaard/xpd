# pymoo

The pymoo pack catches removed and changed pymoo 0.6 APIs, broken `Problem._evaluate` shapes, ignored `minimize` options, unseeded runs, and misconfigured hypervolume or decision weights on files that import [pymoo](https://pymoo.org/). Rules fire only on calls and subclasses bound to `pymoo` imports.

## Older releases

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

- objectives-stacked-by-row
- evaluate-returns-value
- constraint-not-declared
- evaluate-indexing
- elementwise-runner

## Running an algorithm

- minimize-ignored-keyword
- algorithm-read-after-minimize
- reused-algorithm-without-copy
- unread-history
- moead-arguments

## Random numbers

- unseeded-run
- numpy-seed-ignored

## Indicators and decisions

- hv-reference-point
- decision-weights
