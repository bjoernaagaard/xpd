These keywords and argument values were removed from SciPy and raise TypeError on SciPy 1.18.1. Use the replacement that the message names, such as `rtol=` for `tol=`.

```diff lang="python"
-x, info = cg(A, b, tol=1e-8)
-w = linalg.eigh(a, eigvals=(0, 2), eigvals_only=True)
-dist, idx = tree.query(points, k=3, n_jobs=-1)
+x, info = cg(A, b, rtol=1e-8)
+w = linalg.eigh(a, subset_by_index=(0, 2), eigvals_only=True)
+dist, idx = tree.query(points, k=3, workers=-1)
```

## What the rule reports

The rule reports keywords and argument values that SciPy removed and that raise on SciPy 1.18.1.

| Function | Removed form | Fix |
| --- | --- | --- |
| `cg`, `gmres` and the other iterative solvers of `scipy.sparse.linalg` | `tol=`, `atol='legacy'`, a fourth positional argument | `rtol=` for `tol=` |
| `gmres` | `restrt=` | `restart=` |
| `linalg.solve` | `sym_pos=` | `assume_a='pos'` for `sym_pos=True` |
| `linalg.eigh`, `linalg.eigvalsh` | `eigvals=`, `turbo=` | `subset_by_index=` for `eigvals=` |
| `linalg.pinv`, `linalg.pinvh` | `cond=`, `rcond=` | none |
| `integrate.simpson` | `even=` | deletes it |
| `integrate.cumulative_trapezoid` | an `initial` other than 0 | none |
| the query methods of `KDTree` and `cKDTree` | `n_jobs=` | `workers=` |
| `stats.kendalltau` | `initial_lexsort=` | deletes it |
| `stats.iqr`, `stats.median_abs_deviation` | `scale='raw'` | `scale=1.0` |
| `optimize.fmin_l_bfgs_b` | `iprint=`, `disp=` | deletes them |
| `spatial.distance.cdist`, `spatial.distance.pdist` | the metric `'wminkowski'` | none |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `rtol=` for `tol=` on the sparse.linalg solvers, `restart=` for gmres `restrt=`, `assume_a='pos'` for linalg.solve `sym_pos=True`, `subset_by_index=` for eigh and eigvalsh `eigvals=`, `workers=` for `n_jobs=` on the cKDTree query methods, and `scale=1.0` for iqr `scale='raw'`, and it deletes kendalltau `initial_lexsort=` and fmin_l_bfgs_b `iprint=` and `disp=`. On the last SciPy that takes the old form, from 1.8.1 to 1.17.1, each new form gives a result with an equal pickle, and fmin_l_bfgs_b prints nothing either way. Unsafe: it deletes simpson `even=`, because `even='avg'` gives another result, and it writes `workers=` for `n_jobs=` on KDTree and `scale=1.0` for median_abs_deviation `scale='raw'`, because neither ever took the old form. The other forms have no fix, because the call already passes the new keyword, or the message names two rewrites, or the value the new form needs is not in the file.
