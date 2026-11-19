`np.where(x > 0, True, False)` copies a mask that is already an array of booleans, which was 17 times slower. Use the comparison itself.

```diff lang="python"
-relu = np.where(z > 0, z, 0)
+relu = np.maximum(z, 0)
```

## What the rule reports

The rule reports `np.where(x > 0, True, False)`, `np.where(x > 0, 1, 0)`, `np.where(x > 0, x, 0)`, `np.where(x < 0, 0, x)` and `np.arange(len(x))[x > 1]`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.asarray(x > 0)`, `np.asarray(x > 0, dtype=int)`, `np.maximum(x, 0)`, `np.flatnonzero(x > 1)`.
