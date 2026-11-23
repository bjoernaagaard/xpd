CVXPY interprets `*` between non-scalar expressions as matrix
multiplication. This use of the operator has been deprecated since 1.1.

```diff lang="python"
-prediction = coefficients * weights
+prediction = coefficients @ weights
```

## What the rule reports

The rule checks pairs of CVXPY vectors and matrices with compatible
literal dimensions from a call or an unambiguous local assignment. It
reads shapes passed by position or as `shape=`. For matrix multiplication,
the last left dimension must equal the first right dimension. The rule
leaves scalar, singleton, incompatible, and unknown shapes alone.

## How to fix it

Use `@` for a matrix product. For an elementwise product, use
`cp.multiply(coefficients, weights)` with broadcast-compatible shapes.

[CVXPY operator guide](https://www.cvxpy.org/tutorial/functions/index.html#operators)
