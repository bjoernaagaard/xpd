`flatten()` and `copy()` on an array that is already a new copy copy it again. Use `ravel()` or drop the extra copy.

```diff lang="python"
-flat = np.concatenate([a, b]).flatten()
+flat = np.concatenate([a, b]).ravel()
```

## What the rule reports

The rule reports `.flatten()` on the result of a NumPy call that returns a new array, `x.copy().astype(t)` and `np.array(x).copy()`. Each one copies an array that is already a new copy.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.abs(m).ravel()`, `x.astype(np.float32)`, `np.array(x)`. ravel can return a view of an output array that was passed by position, and the other two can change the memory order.
