`linalg.det` overflows or underflows for a large matrix, so its log is inf or -inf. Use `np.linalg.slogdet(a)[1]`.

```diff lang="python"
-logdet = np.log(linalg.det(cov))
+logdet = np.linalg.slogdet(cov)[1]
```

## What the rule reports

The rule reports `np.log(linalg.det(a))`, `np.log(np.abs(linalg.det(a)))` and
`np.log(abs(linalg.det(a)))`, where `linalg` is `scipy.linalg` or `det` was imported from it. On two
covariance matrices the old form gave -inf and inf, where `slogdet` lost less than one digit, as
the table shows. `np.log(np.linalg.det(a))` belongs to the NumPy rule
[`log-of-det`](../numpy/log-of-det.md).

| Covariance matrix | `np.log(linalg.det(a))` | Digits that `slogdet` lost |
| --- | --- | --- |
| 120 features at scale 1e-3 | -inf | 0.54 |
| 80 features at scale 1e4 | inf | 0.27 |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.linalg.slogdet(a)[1]`.
