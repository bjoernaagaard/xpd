CVXPY 1.x uses column-major order when `reshape`, `vec`, or
`Expression.flatten` receives no order or receives `None`. From CVXPY 1.6
these calls emit a FutureWarning. NumPy defaults to row-major order.

```diff lang="python"
-vector = cp.vec(matrix)
+vector = cp.vec(matrix, order="F")
```

## What the rule reports

The rule checks CVXPY's `reshape` and `vec` functions and the `reshape`
and `flatten` methods of locally resolved variables, parameters, and
constants. It finds `order=None` in any keyword position and through
parentheses. Argument unpacking is outside the rule. The omitted-order fix
checks calls without comments between arguments so the edit can preserve
their layout.

## How to fix it

`xpd check --fix` writes explicit `"F"` to preserve the CVXPY 1.x order.
Write `"C"` if the model uses NumPy's row-major order. With
`[[1, 2, 3], [4, 5, 6]]`, those orders give `[1, 4, 2, 5, 3, 6]` and
`[1, 2, 3, 4, 5, 6]`, respectively.

[CVXPY changelog on reshape order](https://www.cvxpy.org/updates/#update-on-reshape-order)
