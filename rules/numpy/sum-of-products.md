`np.sum(a * b)` builds the array of products to add it up, which was 2.4 times slower. Use `np.vdot` or `np.dot`.

```diff lang="python"
-energy = np.sum(signal ** 2)
+energy = np.vdot(signal, signal)
```

## What the rule reports

The rule reports `np.sum(a * b)`, `np.sum(x ** 2)`, `np.sum(np.square(x))` and `np.mean((a - b) ** 2)`, but not when an operand is a number, a comparison, a conjugate or a new axis.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.vdot(a, b)`, `np.vdot(x, x)`. The mean of squared differences has no fix.
