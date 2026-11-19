For two 2-D arrays, NumPy's docs prefer the `@` operator to `np.dot`. Write `a @ b`.

```diff lang="python"
-cov = np.dot(X.T, X) / n
+cov = X.T @ X / n
```

## What the rule reports

The rule reports `np.dot(a, b)`, because the docs of `np.dot` prefer `matmul` or `a @ b` for two 2-D arrays.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `a @ b`, with parentheses around an operand such as `a + 1`, and around the product where it is part of a larger operand, as in `(a @ b).T`. `@` raises on a scalar and on two Python lists, and on arrays of more than two dimensions it works on stacks of matrices, where `np.dot` does not.
