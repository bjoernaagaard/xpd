CVXPY 1.x uses column-major order when `reshape`, `vec`, or
`Expression.flatten` receives no order or receives `None`. From CVXPY 1.6
these calls emit a FutureWarning. NumPy defaults to row-major order.

```diff lang="python"
-vector = cp.vec(matrix)
+vector = cp.vec(matrix, order="F")
```

## What the rule reports

Direct calls to CVXPY's `reshape` and `vec`, and `flatten` on a locally
resolved CVXPY variable, parameter, or constant. The rule skips argument
unpacking and comments between arguments.

## How to fix it

`xpd check --fix` writes explicit `"F"` to preserve the CVXPY 1.x order.
Write `"C"` if the model uses NumPy's row-major order. With
`[[1, 2, 3], [4, 5, 6]]`, those orders give `[1, 4, 2, 5, 3, 6]` and
`[1, 2, 3, 4, 5, 6]`, respectively.

[CVXPY changelog on reshape order](https://www.cvxpy.org/updates/#update-on-reshape-order)
