The determinant can overflow to inf or underflow to 0 before the log is taken. Use `np.linalg.slogdet(a).logabsdet`.

```diff lang="python"
-log_det = np.log(np.linalg.det(cov))
+log_det = np.linalg.slogdet(cov).logabsdet
```

## What the rule reports

The rule reports `np.log(np.linalg.det(a))` and `np.log(np.abs(np.linalg.det(a)))`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.linalg.slogdet(a).logabsdet`. Needs NumPy 2.0.
