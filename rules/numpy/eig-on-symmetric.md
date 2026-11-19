`eig` treats a symmetric matrix as a general one, which was 3.6 times slower and returns complex values. Use `np.linalg.eigh`.

```diff lang="python"
-values, vectors = np.linalg.eig(np.cov(data, rowvar=False))
+values, vectors = np.linalg.eigh(np.cov(data, rowvar=False))
```

## What the rule reports

The rule reports `np.linalg.eig` and `np.linalg.eigvals` on a matrix the code made symmetric: `x @ x.T`, `x.T @ x`, `np.dot(x, x.T)`, `np.dot(x.T, x)`, `a + a.T`, `np.cov(...)` and `np.corrcoef(...)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.linalg.eigh(m)`, `np.linalg.eigvalsh(m)`.
