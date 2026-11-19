Summing squares or products along the last axis builds a temporary array, which was 2.8 times slower. Use `np.vecdot`.

```diff lang="python"
-squared_length = np.sum(points ** 2, axis=-1)
+squared_length = np.vecdot(points, points)
```

## What the rule reports

The rule reports `np.sum(x ** 2, axis=-1)` and `np.sum(np.square(x), axis=-1)`. It also reports `np.sum(a * b, axis=-1)` and `np.sum(a.conj() * b, axis=-1)` where `a` and `b` are names or subscripts. `np.vecdot` multiplies and sums in one pass without the temporary array, and ran 2 to 3.5 times faster on 1,000,000 values.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.vecdot(x, x)` and `np.vecdot(a, b)`. Floats can differ in the last bits, int8 products can wrap where `np.sum` counted in int64, two bool arrays give bool, and vecdot conjugates a complex first argument. Needs NumPy 2.0.
