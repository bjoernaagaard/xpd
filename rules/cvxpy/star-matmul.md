CVXPY interprets `*` between non-scalar expressions as matrix
multiplication. This use of the operator has been deprecated since 1.1.

```diff lang="python"
-prediction = coefficients * weights
+prediction = coefficients @ weights
```

## What the rule reports

A product of two CVXPY variables or parameters with literal shapes whose
dimensions are all at least two. The rule leaves scalar, singleton, and
unknown shapes alone.

## How to fix it

Use `@` for a matrix product. For an elementwise product, use
`cp.multiply(coefficients, weights)` with broadcast-compatible shapes.

[CVXPY operator guide](https://www.cvxpy.org/tutorial/functions/index.html#operators)
