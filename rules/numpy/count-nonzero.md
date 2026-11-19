Summing a comparison or taking the length of `np.where` counts matches the slow way. Call `np.count_nonzero`.

```diff lang="python"
-changed = np.sum(before != after)
+changed = np.count_nonzero(before != after)
```

## What the rule reports

The rule reports `np.sum(x > 0)`, `len(np.where(c)[0])`, `len(np.nonzero(c)[0])`, `len(np.flatnonzero(c))`, `len(x[x > 0])` and `np.sum(x > 2) > 0`. In a file that imports pandas, the pandas rule [`len-of-filter`](../pandas/len-of-filter.md) reports `len(x[x > 0])` instead, because `x` may be a frame or a series.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `np.any(x > 2)`. The unsafe fix writes `np.count_nonzero(x > 0)`, also for `len(x[x > 0])`, which gave a Python int.
