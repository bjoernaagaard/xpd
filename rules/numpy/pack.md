# NumPy

The NumPy pack catches silent bugs, NumPy 2.5 deprecations and removals, slow Python element loops, and suboptimal array math on files that import `numpy`. Examples assume `import numpy as np`.

## Code that gives a wrong result or raises on NumPy 2.5

- compare-to-nan
- argument-in-wrong-slot
- percentile-of-fraction
- positional-tolerance
- arange-float-step
- randint-single-value
- result-is-none
- result-discarded
- write-to-copy
- array-truth-value
- mask-operator-precedence
- int-mask-as-index
- iterator-to-array
- nan-to-int
- read-uninitialized
- array-default-argument
- append-flattens
- compare-reductions
- where-tuple-length

## Code that raises or warns on NumPy 2.5

- removed-in-numpy-2
- deprecated-in-numpy-2

## Python work per value on arrays

- element-loop
- growth-in-loop
- per-value-comprehension
- builtin-on-array
- np-vectorize
- recurrence-loop
- slice-fill-loop
- mask-per-label
- random-draw-loop

## Arrays built, copied and stored the slow way

- array-construction
- needless-copy
- file-roundtrip

## Math on whole arrays with a faster or more exact function

- sort-then-select
- count-nonzero
- where-shortcut
- expm1-log1p
- logaddexp
- log-of-product
- reduce-last
- power-half
- sum-of-products
- norm-by-hand
- vecdot
- inv-then-multiply
- log-of-det
- eig-on-symmetric
- matmul-diagonal
- einsum-optimize

## The API that NumPy's docs prefer on NumPy 2.5

- legacy-random
- rng-in-loop
- clock-seed
- legacy-polynomial
- almost-equal-assert
- assert-on-arrays
- legacy-testing-helper
- char-module
- dot-on-matrices
- mergesort-kind
- printoptions-restore
- as-strided
- numpy-import-alias
- unpackbits-popcount
