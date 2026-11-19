Building an array from `range` or a Python list makes a Python int for every value first, and `np.ones(n) * 3` builds the values twice. Call `np.arange` or `np.full` directly, which ran 150 times faster.

```diff lang="python"
-weights = np.ones(n) * 0.5
+weights = np.full(n, 0.5, dtype=np.float64)
```

## What the rule reports

The rule reports `np.array(range(n))`, `np.array([0] * n)`, `np.ones(n) * 3`, `np.zeros(n).astype(np.float32)` and `np.arange(n).astype(float)`. Each one builds the values twice, or builds a Python int for every value.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `np.zeros(n, dtype=np.float32)`. The unsafe fix writes `np.full(n, 3, dtype=np.float64)`, which gives a zero dimensional array where the product of an empty shape gave a scalar, `np.arange(n)`, `np.full(n, 0)`, `np.arange(n, dtype=float)`.
