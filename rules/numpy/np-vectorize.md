`np.vectorize`, `np.frompyfunc` and `np.apply_along_axis` still call a Python function once per value or slice. Where the function is arithmetic or a condition, write it on whole arrays.

```diff lang="python"
-@np.vectorize
 def clipped(x):
-    return min(x, 3.0)
+    return np.minimum(x, 3.0)
```

## What the rule reports

The rule reports `np.vectorize`, `np.frompyfunc` and `np.apply_along_axis`, whether called or used as a decorator.

## How to fix it

xpd has no automatic fix, so make the change by hand.
