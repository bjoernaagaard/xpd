Computing a whole matrix product to read its diagonal or trace was 34 times slower than needed. Use `np.einsum` to compute only the diagonal.

```diff lang="python"
-leverage = np.diag(X @ cov @ X.T)
+leverage = np.einsum("ij,ij->i", X @ cov, X)
```

## What the rule reports

The rule reports `np.diag(a @ b)`, `np.diag(np.dot(a, b))`, `np.diagonal(a @ b)`, `np.diag(a @ b.T)`, `np.trace(a @ b)` and `np.trace(np.dot(a, b))`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.einsum("ij,ji->i", a, b)`, `np.einsum("ij,ij->i", a, b)`, `np.einsum("ij,ji->", a, b)`.
