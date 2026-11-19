These names, modules and keywords were removed between NumPy 1.24 and 2.5, so the code raises. Use the replacement that the message and the table show.

```diff lang="python"
-values = np.array(rows, dtype=np.float_, copy=False)
-low = np.quantile(values, 0.1, interpolation="nearest")
-clean = np.where(values > 0, values, np.NaN)
-warning = np.ComplexWarning
+values = np.asarray(rows, dtype=np.float64)
+low = np.quantile(values, 0.1, method="nearest")
+clean = np.where(values > 0, values, np.nan)
+warning = np.exceptions.ComplexWarning
```

## What the rule reports

The rule reports the names, methods, modules and keywords that NumPy 1.24 through 2.5 removed, and its message says what to write instead. The rule leaves alone code under a NumPy version check, a `hasattr(np, ...)` check, or a `try` that catches ImportError or AttributeError. Such code exists to support old NumPy.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes the builtin for `np.int`, `np.float`, `np.complex`, `np.object`, `np.str` and `np.unicode`, `np.float64` for `np.float_` and the other scalar aliases, `np.nan` and `np.inf` for `np.NaN`, `np.Inf` and their kin, `np.prod`, `np.cumprod`, `np.any`, `np.all`, `np.round` and `np.vstack` for the renamed functions, `np.exceptions.AxisError` and the other warnings and errors, `np.trapezoid`, `np.sort(a, axis=0)`, `a.item()`, `np.pad` for `np.lib.pad` and the other functions that `np.lib` re-exported, a shape by position for `newshape=`, `method=` for `interpolation=` and `minlength=0`. The unsafe fix writes `np.exceptions.RankWarning` and `np.exceptions.DTypePromotionError`, because no NumPy release has both the old and the new place, `"S"` for the dtype code `"a"`, because a `dtype=` keyword of another library can give the text another meaning, deleting `fix_imports=`, `bias=` and `ddof=`, because a call in the deleted value no longer runs, `np.ptp(a)`, `a.tobytes()` and `a[i, j] = v` for `a.itemset`, because the pattern cannot tell an array from another object with a method of that name, `np.isin` for `np.in1d`, `density=` for `normed=`, `np.asarray` for `np.array(x, copy=False)`, `np.frombuffer` for `np.fromstring`, deleting `style=`, and the module path for `numpy.linalg.linalg`, `numpy.fft.helper`, `numpy.testing.utils` and the private modules of `numpy.lib`, imported or read by attribute. Also unsafe, because the old form raises on NumPy 2.5: `(-np.inf)` and `(-0.0)` where `np.NINF` and `np.NZERO` are part of a larger operand, `(x + 1).item()` for `np.asscalar(x + 1)`, `from numpy.exceptions import` for an import of the moved errors and warnings, `from numpy.lib.array_utils import byte_bounds`, `a.flat[4] = v` for `a.itemset(4, v)`, `newshape=` moved by position before the other keywords, and `copy=None` for the other `np.array(copy=False)` calls. The other forms have no fix.
