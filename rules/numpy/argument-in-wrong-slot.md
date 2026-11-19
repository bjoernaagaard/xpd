These calls put an argument where NumPy expects another one, such as a second size where `np.zeros` expects the dtype, so they raise or give a wrong result. Pass the shape as one tuple, or pass the argument in its own place.

```diff lang="python"
-grid = np.zeros(3, 4)
+grid = np.zeros((3, 4))
```

## What the rule reports

The rule reports `np.zeros(3, 4)`, `np.full(3, 4, 0)`, `np.reshape(a, 2, 3)`, `np.vstack(a, b)`, `np.concatenate(a, [4, 5])`, `np.random.rand((3, 4))`, `np.random.random(3, 4)`, `np.linspace(0, 1, 0.1)` and `np.quantile(x, 95)`. Each call puts an argument in the place of another parameter, so it raises.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.zeros((3, 4))`, `np.hstack((a, b, a))` with any keywords after the tuple, and `np.percentile` with the same arguments for a quantile call with more arguments. The linspace form has no fix, because the code does not show whether a count or a step was meant.
