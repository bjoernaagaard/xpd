These names warn on SciPy 1.18.1, often because they come from a private module. Import the same object from the public place that the message names.

```diff lang="python"
-from scipy.ndimage.filters import gaussian_filter
-from scipy.stats.stats import pearsonr
-smooth = scipy.sparse.linalg.isolve.cg(A, b)
-cell = scipy.spatial.tsearch(tri, points)
-D = scipy.spatial.distance_matrix(X, Y)
+from scipy.ndimage import gaussian_filter
+from scipy.stats import pearsonr
+smooth = scipy.sparse.linalg.cg(A, b)
+cell = tri.find_simplex(points)
+D = scipy.spatial.distance.cdist(X, Y)
```

## What the rule reports

The rule reports names that warn with `DeprecationWarning` on SciPy 1.18.1, and its message
names the replacement that SciPy's warning names. The names are:

- every name taken from a private module, such as `scipy.stats.stats`, `scipy.linalg.basic`,
  `scipy.ndimage.filters` or `scipy.sparse.linalg.isolve`, and an import of such a module
- `scipy.odr`, which SciPy 1.19 removes
- calls of `minkowski_distance`, `minkowski_distance_p`, `distance_matrix`, `tsearch`,
  `lagrange`, `pade`, `approximate_taylor_polynomial` and `py_vq`

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes the public namespace for a name taken from one of the 107 private modules that SciPy deprecated, such as `scipy.sparse.csr_matrix` for `scipy.sparse.csr.csr_matrix` and `from scipy.ndimage import gaussian_filter` for `from scipy.ndimage.filters import gaussian_filter`, which give the same object, and `tri.find_simplex(xi)` for `scipy.spatial.tsearch(tri, xi)`, which is what `tsearch` returns. Unsafe: `vq` for `scipy.cluster.vq.py_vq`, and `vq as py_vq` in an import, because `vq` returns the codes as int32 where `py_vq` returned int64, and float32 distances for float32 input. The other forms have no fix, because the public module lacks the name or holds another object, an import of a whole private module would change every later use, or the replacement takes other arguments or is another package.
