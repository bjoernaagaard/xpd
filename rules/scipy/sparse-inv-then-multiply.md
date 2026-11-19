The inverse of most sparse matrices is dense, so `inv(A) @ b` builds a huge array. Call `spsolve(A, b)`, which was 260 times faster.

```diff lang="python"
-x = spla.inv(A) @ b
+x = spla.spsolve(A, b)
```

## What the rule reports

The rule reports `scipy.sparse.linalg.inv(A) @ b` and `inv(A).dot(b)`, because the inverse of
most sparse matrices is dense. On a tridiagonal matrix of 1,000 rows, whose inverse holds 1,000,000
values, `spsolve` ran 260 to 270 times faster.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `spla.spsolve(A, b)` when `inv` is reached through its module. The last bits change, and a `b` of shape `(n, 1)` gives a 1-D result. `inv` imported by name is fixed only in a file that imports `spsolve` from `scipy.sparse.linalg`.
