A number from a float64 array or a Series is a float already, and so is arithmetic on it. Drop the `float()`, and keep it only for text, an int, or a float32.

```diff lang="python"
 def cheapest(prices: np.ndarray, sold: np.ndarray) -> tuple[float, float]:
     best = prices.argmin()
-    return float(prices[best]), float(sold[best] * 2)
+    return prices[best], sold[best] * 2
```

## What the rule reports

The rule reports `float()` around a subscript, a name, an attribute or arithmetic, in a file
that imports numpy or pandas. It leaves alone a string and a value whose name points at text,
such as a regex group, a split, a header or the environment.

## How to fix it

`xpd check --fix --unsafe-fixes` unwraps `float(value)` to `value` (or `(value)` for arithmetic expressions). It is unsafe because the file does not prove whether the value is text, an integer that needs conversion, or a `float32`.

A number taken from a float64 array or a Series is an `np.float64`, and so is arithmetic on it.
On NumPy 2.5.5 and pandas 3.1.1, `isinstance(value, float)` was True for `a[0]`, `a[0] * 3`,
`s.iloc[0]`, `s.sum()` and `s.max()`. Keep the call in three cases: the value is text, the value is an int that has to become
a float, or the array holds float32.
