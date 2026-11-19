SciPy keeps these names only for old code. Use the replacement that the message names.

```diff lang="python"
-totals = ndimage.sum(image, labels, index)
-stat, p = stats.ranksums(x, y)
+totals = ndimage.sum_labels(image, labels, index)
+u, p = stats.mannwhitneyu(x, y, use_continuity=False, method="asymptotic")
```

## What the rule reports

The rule reports names that SciPy keeps for old code:

| Name | What SciPy says | Write instead |
| --- | --- | --- |
| `ndimage.sum` | kept for backwards compatibility | `ndimage.sum_labels` |
| `stats.ranksums` | legacy | `stats.mannwhitneyu`, which gives the same p-value and the U statistic in place of z |

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `ndimage.sum_labels` for `ndimage.sum`, whose body only calls sum_labels. The other forms have no fix.
