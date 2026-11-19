`np.exp(x) - 1` and `np.log(1 + x)` lose digits to rounding when `x` is near 0. Use `np.expm1` and `np.log1p`.

```diff lang="python"
-log_survival = np.log(1 - p)
+log_survival = np.log1p(-p)
```

## What the rule reports

The rule reports `np.exp(x) - 1`, `1 - np.exp(x)`, `np.log(1 + x)`, `np.log(x + 1)` and `np.log(1 - p)`, but not on a number literal.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.expm1(x)`, `-np.expm1(x)`, `np.log1p(x)`, `np.log1p(-p)`.
