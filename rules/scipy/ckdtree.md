`cKDTree` is the same as `KDTree`, and SciPy keeps the name only for old code. Use `KDTree` unless the code must run on SciPy older than 1.6.

```diff lang="python"
-tree = cKDTree(points)
+tree = KDTree(points)
```

## What the rule reports

The rule reports a call to `cKDTree`, whose docstring says it is functionally identical to
`KDTree` and kept only for backward compatibility.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `KDTree` for `cKDTree`, with the same arguments. The default `leafsize` differs, 16 against 10, so a query can return another one of two equally near points, and the tree's node classes differ. `cKDTree` imported by name is fixed only in a file that imports `KDTree` from `scipy.spatial`. Skipped on SciPy older than 1.6.
