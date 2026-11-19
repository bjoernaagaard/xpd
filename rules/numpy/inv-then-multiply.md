Multiplying by an inverse loses more digits than solving, and was 3.8 times slower. Call `np.linalg.solve(a, b)`.

```diff lang="python"
-beta = np.linalg.inv(X.T @ X) @ X.T @ y
+beta = np.linalg.solve(X.T @ X, X.T @ y)
```

## What the rule reports

The rule reports `np.linalg.inv(a) @ b`, `np.linalg.inv(a).dot(b)`, `np.dot(np.linalg.inv(a), b)` and `np.linalg.inv(a) @ b @ c`, as in the normal equations.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.linalg.solve(a, b)`, `np.linalg.solve(a, b @ c)`.
