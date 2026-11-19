# NumPy

The NumPy rules find code that gives a wrong result or raises on NumPy 2.5, and code that has a
faster or more exact form. They run on files that import `numpy`, and examples assume
`import numpy as np`. Each claim in a message was measured on NumPy 2.5.5, and the table says what each claim means.

| Claim | What it means on NumPy 2.5.5 |
| --- | --- |
| a form is removed | it raises |
| a form is wrong | it gives the wrong value |
| a form is faster | the median of 7 runs on one core, at 1,000,000 values unless the rule says otherwise |

A fix that writes `np.` applies only in a file with `import numpy as np`, and a fix that writes
`assert_allclose` only in a file with `from numpy.testing import assert_allclose`. A safe fix returns a
value of the same dtype that is equal bit for bit, and the **Fix:** line of an unsafe fix names
what can differ.

xpd skips a rule, or one form of a rule, that asks for an API newer than the NumPy in the
project's virtual environment (venv). When the venv has no NumPy, or there is no venv, every rule runs.

| Rule | Runs on |
| --- | --- |
| [`char-module`](char-module.md), [`log-of-det`](log-of-det.md), [`unpackbits-popcount`](unpackbits-popcount.md) and [`vecdot`](vecdot.md) | NumPy 2.0 and later |
| some forms of [`removed-in-numpy-2`](removed-in-numpy-2.md) | NumPy 1.22, 1.25 or 2.0 and later, by form |
| some forms of [`deprecated-in-numpy-2`](deprecated-in-numpy-2.md) | NumPy 2.0 and later |
| every other NumPy rule | every NumPy version |

## Code that gives a wrong result or raises on NumPy 2.5

Fix every report in the group, because the reported code gives a wrong value without an error,
or raises on every input. Every fix in the group is unsafe, because it changes the result to the
one the code meant, except the fix of `positional-tolerance`, which only names the argument.

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

Replace each name, method, module or keyword that NumPy removed or deprecated with the form that
the rule's message names, because the old form raises or warns on NumPy 2.5.

- removed-in-numpy-2
- deprecated-in-numpy-2

## Python work per value on arrays

Write one expression on whole arrays in place of code that calls Python once per value or copies
the whole array once per pass, because the whole array form ran up to 530 times faster. The
message of each rule names the whole array form. The table shows the speed that the message
reports, as medians of 7 runs on one core with NumPy 2.5.5, on float64 and int64.

| Rule | Write instead | Faster by |
| --- | --- | --- |
| [`element-loop`](element-loop.md) | one expression on whole arrays | 70 to 530 times on 1,000,000 values |
| [`growth-in-loop`](growth-in-loop.md) | a list, and one `np.concatenate` after the loop | 130 to 170 times for 1,000 passes over 1,000,000 values |
| [`per-value-comprehension`](per-value-comprehension.md) | the expression on `np.asarray(...)` | 60 to 470 times on an array of 1,000,000 values, and 2 to 24 times on a list |
| [`builtin-on-array`](builtin-on-array.md) | the method, such as `.sum(axis=0)` | 37 to 47 times on 1,000,000 values |
| [`np-vectorize`](np-vectorize.md) | code on whole arrays, such as `np.where` or `np.select` | not measured |

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

Build, copy and store each array with the NumPy function that the rule names, because it ran 1.6
to 880 times faster on 1,000,000 values. The table shows the speed that the message of each rule
reports.

| Rule | Write instead | Faster by |
| --- | --- | --- |
| [`array-construction`](array-construction.md) | `np.arange`, `np.zeros` or `np.full` | 150 times for `np.arange` |
| [`needless-copy`](needless-copy.md) | `.ravel()` | 1.6 to 1.8 times |
| [`file-roundtrip`](file-roundtrip.md) | `np.save` and `np.load` | 27 to 880 times to read, and 420 to 510 times to write |

- array-construction
- needless-copy
- file-roundtrip

## Math on whole arrays with a faster or more exact function

Call the NumPy function that each rule names, because it does the same work without a full sort,
a temporary array or a step that loses digits. Every rule in the group is a warning. The table
shows the speed that the message of each rule reports, measured with NumPy 2.5.5 on one core,
with BLAS and LAPACK, the linear algebra libraries that NumPy calls, on one thread. The other rules in the group give a more exact result and have no
measured speed.

| Rule | Reported form | Size |
| --- | --- | --- |
| [`sort-then-select`](sort-then-select.md) | 39 times slower than `np.min` | 1,000,000 values |
| [`count-nonzero`](count-nonzero.md) | 1.7 times slower than `np.count_nonzero` | 1,000,000 values |
| [`where-shortcut`](where-shortcut.md) | 17 times slower than `np.asarray` | 1,000,000 values |
| [`reduce-last`](reduce-last.md) | 8 times slower than `np.sum` | 1,000,000 values |
| [`power-half`](power-half.md) | 2 times slower than `np.sqrt` | 1,000,000 values |
| [`sum-of-products`](sum-of-products.md) | 2.4 times slower than `np.vdot` | 1,000,000 values |
| [`norm-by-hand`](norm-by-hand.md) | 3.9 times slower than `np.linalg.norm` | 1,000,000 values |
| [`vecdot`](vecdot.md) | 2 to 3.5 times slower than `np.vecdot` | 1,000,000 values |
| [`inv-then-multiply`](inv-then-multiply.md) | 3.8 times slower than `np.linalg.solve` for a vector | a 1,000 by 1,000 matrix |
| [`eig-on-symmetric`](eig-on-symmetric.md) | 3.6 times slower than `np.linalg.eigh` | a 1,000 by 1,000 matrix |
| [`matmul-diagonal`](matmul-diagonal.md) | 34 times slower than `np.einsum` | 1,000 by 1,000 matrices |
| [`einsum-optimize`](einsum-optimize.md) | 339 times slower than `optimize=True` | three 100 by 100 matrices |

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

Write the form that NumPy's docs prefer in place of each old form that the rules report, although
the old form still runs without a warning on NumPy 2.5.5. A rule reports an old form when NumPy's
docs call it legacy, not recommended or kept for backwards compatibility, or when the newer form
ran at least 1.5 times faster. A rule that states only a preference of the docs is info, and the
rest are warnings, as the table shows. xpd skips the rules that name an API added in NumPy 2.0 on
an older NumPy.

| Severity | Rules |
| --- | --- |
| warning | [`legacy-random`](legacy-random.md), [`rng-in-loop`](rng-in-loop.md), [`clock-seed`](clock-seed.md) and [`unpackbits-popcount`](unpackbits-popcount.md) |
| info | the other 10 rules of the group |

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
