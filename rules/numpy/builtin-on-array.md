Python's `sum`, `min`, `max`, `sorted` and `list` walk an array one value at a time, which was 37 to 47 times slower. Call the NumPy method instead.

```diff lang="python"
-peak = max(np.abs(signal))
+peak = np.abs(signal).max(axis=0)
```

## What the rule reports

The rule reports `sum`, `min`, `max`, `sorted` and `list` around a NumPy call, as in `sum(np.abs(a))`, or around a comparison with a NumPy call, as in `sum(np.abs(a) > 1)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.abs(a).sum(axis=0)`, `np.abs(a).max(axis=0)`, `np.sort(np.abs(a))` for `sorted`, which gives an array, and `np.abs(a).tolist()` for `list`, which gives Python numbers.
