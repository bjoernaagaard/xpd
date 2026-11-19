`float()`, `int()`, `str()` or `bool()` around a value that already has that type does nothing. Drop the call.

```diff lang="python"
 def summary(prices: pd.Series, day: date) -> tuple[str, int, float, bool]:
-    return (
-        str(day.isoformat()),
-        int(prices.nunique()),
-        float(prices.std()),
-        bool(prices.empty),
-    )
+    return day.isoformat(), prices.nunique(), prices.std(), prices.empty
```

## What the rule reports

The rule reports `float()`, `int()`, `str()` and `bool()` around a value that has that type
already. The table lists what it reports and what it leaves alone.

| Call | Reported around | Left alone |
| --- | --- | --- |
| `float()` | `mean`, `median`, `std`, `var`, `sem`, `skew`, `kurt`, `corr` and `autocorr` without an axis, in a file that imports numpy or pandas. `np.sqrt`, `np.exp`, `np.log`, `np.hypot`, `np.mean`, `np.median`, `np.percentile`, `np.quantile`, `np.average`, `np.interp`, `np.trapezoid`, `np.linalg.norm` and the `nan` forms without an axis. A division with `/` | A call with `axis=`, and a file that imports torch, jax, tensorflow, xarray, cupy, dask, mlx, polars or sympy |
| `int()` | `nunique()`, `.size`, `.ndim` and `.shape[0]` | `sum`, `count`, `argmax` and `np.count_nonzero`, which return `np.int64`. `len(x)` and `round(x)`, which ruff's `RUF046` reports |
| `str()` | An f-string, and `isoformat`, `strftime`, `format`, `join`, `strip`, `lower`, `upper`, `title`, `removeprefix`, `removesuffix`, `read_text`, `decode`, `hexdigest` and `as_posix` | A plain string, which ruff's `UP018` reports |
| `bool()` | `not x`, `x in y`, `x is y`, `isinstance`, `any`, `all`, `startswith`, `endswith`, `exists`, `is_file`, `is_dir`, `.empty` and `.is_unique` | A comparison such as `a < b`, which returns `np.bool` on NumPy values |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes the value without `int()`, `str()` or `bool()`. With the unsafe fix, the value without `float()`, because the mean of a float32 array is an `np.float32`, which is not a float. `float()` around a division has no fix.

xpd applies the replacement with `--fix`, and the one for `float()` with `--fix --unsafe-fixes`.

Each call converts a value to the type it has. The table shows what Python returned on NumPy
2.5.5 and pandas 3.1.1, and ty 0.0.105 accepted `return x.mean()` in a function that returns
`float`.

| Expression | Type | `isinstance` of the type in the name |
| --- | --- | --- |
| `a.mean()`, `s.median()`, `s.quantile(0.5)`, `s.corr(s)` | `np.float64` | `float`: True |
| `np.sqrt(a[0])`, `np.median(a)`, `np.percentile(a, 50)`, `np.interp(1.5, a, a)`, `np.linalg.norm(a)` | `np.float64` | `float`: True |
| `a.astype("float32").mean()` | `np.float32` | `float`: False |
| `s.nunique()`, `s.size`, `a.ndim`, `a.shape[0]` | `int` | `int`: True |
| `(s > 1).sum()`, `s.count()`, `a.argmax()`, `np.count_nonzero(a)` | `np.int64` | `int`: False |
| `s.empty`, `s.is_unique` | `bool` | `bool`: True |
| `(s > 1).any()`, `a[0] < 1` | `np.bool` | `bool`: False |

`int()` around `sum`, `count` and `argmax` does change the type, and ty rejects
`return x.argmax()` in a function that returns `int`, so the rule leaves those calls alone. An
`np.int64` works as an index and in arithmetic without a conversion. Convert it once, where the
value is written to JSON or returned as an `int`, and not at every step on the way.
