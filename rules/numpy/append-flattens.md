Without `axis`, `np.append` and `np.insert` flatten both arrays, so a row you add becomes loose values in one dimension. Pass `axis=0` to add rows or `axis=1` to add columns.

```diff lang="python"
-grown = np.append(m, [[5, 5]])
+grown = np.append(m, [[5, 5]], axis=0)
```

## What the rule reports

The rule reports `np.append(m, [[5, 5]])` and `np.insert(m, 0, [[5, 5]])` without `axis`. Both calls flatten the array, so the row becomes separate values in one dimension.

## How to fix it

xpd has no automatic fix, so make the change by hand.
