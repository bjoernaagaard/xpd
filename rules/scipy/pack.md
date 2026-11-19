# SciPy

The SciPy rules report code that raises or warns on SciPy 1.18.1, code that loses digits, and
SciPy calls made once per value, in each file that imports `scipy`. The examples assume
`import numpy as np`.

When a message uses a word from the table, the word means the result of a check on SciPy 1.18.1
with NumPy 2.5.5.

| Word in the message | What was measured |
| --- | --- |
| removed, deprecated | The old form raises under `python -W error`. |
| inexact | The old form lost at least 3 decimal digits against mpmath at 50 digits, and the form to write lost fewer than 1. |
| faster | The range over repeated runs on one core, at 1,000,000 values unless the rule says otherwise. The faster form was not slower at 1,000 values. |
| legacy, preferred | SciPy's own source calls the old form legacy or prefers the new one. Such rules have severity info and claim nothing about speed or accuracy. |

A rule reports a name only when the dotted name starts with `scipy`, or when the file's own
imports bind it from `scipy`. A form made only of NumPy names, such as `np.linalg.inv(a) @ b`, belongs to the
[NumPy](../numpy/pack.md) rules. A rule that asks for an API newer than the project's SciPy is skipped.

| Fix | What it promises |
| --- | --- |
| safe | The same object, or a value of the same type that is equal bit for bit. xpd offers no safe fix where the file binds the name again or the call spreads `*args` or `**kwargs`. |
| unsafe | The message names what can differ. |

## Names that are gone or that warn

Replace each name that SciPy removed or deprecated with the name the rule gives, because a removed name raises on import or on the call.

- removed-in-scipy
- deprecated-in-scipy

## Arguments, defaults and legacy functions

Change each argument that SciPy removed, deprecated or ignores, and move each legacy function to the interface that SciPy's documentation names as its replacement.

- removed-keyword
- deprecated-argument
- default-that-warns
- minimize-unknown-option
- minimize-unused-argument
- rng-keyword
- legacy-optimize
- legacy-odeint
- legacy-interpolate
- legacy-signal
- legacy-api

## Forms that keep digits

Use the SciPy function that computes the quantity directly, because the two-step form loses digits or overflows where the direct function doesn't.

- log-of-special
- log-of-dist-method
- cdf-complement
- special-complement
- ppf-of-complement
- comb-by-factorials
- log-of-scipy-det

## SciPy calls made once per value or once per pair

Replace each SciPy call that Python makes once per value, once per pair of points or once per
pass of a loop with one call on whole arrays, which does the same work in compiled code. Each
speedup is a median of 7 runs on one core with SciPy 1.18.1, on float64 and float32, at 1,000,000
values unless the rule names another size.

- distance-per-pair
- callable-metric
- per-value-scipy-call
- query-per-point
- solve-in-loop
- sparse-inv-then-multiply

## Sparse matrices

Build a sparse matrix in a format made for writes, and use the sparse array classes in place of the matrix classes.

- empty-csr-write
- sparse-matrix-api

## The API that SciPy's docs prefer on SciPy 1.18

Use the class or module that SciPy's documentation prefers, because the older name stays only for old code.

- ckdtree
- legacy-fftpack
