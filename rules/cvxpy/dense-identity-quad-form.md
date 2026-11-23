CVXPY's `quad_form(x, np.eye(n))` allocates a dense square identity
matrix to express the sum of the squares of a real vector.

```diff lang="python"
 x = cp.Variable(3)
-loss = cp.quad_form(x, np.eye(3))
+loss = cp.sum_squares(x)
```

## What the rule reports

A default real `Variable` with a literal vector length and an equally
sized `numpy.eye` or `numpy.identity` call with one argument. The variable
must be constructed in the call or bound in the preceding statement.

## How to fix it

Use `cp.sum_squares(x)` to express the same scalar objective without
allocating the matrix. Keep `quad_form` for a quadratic form with a
nontrivial matrix.

[CVXPY performance guide to quadratic objectives](https://www.cvxpy.org/tutorial/performance/index.html#use-cp-sum-squares-for-quadratic-objectives)
