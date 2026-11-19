After `unstack`, a pair without rows becomes a missing value, which turns int64 counts into float64. Pass `fill_value=0`, which keeps the counts int64.

```diff lang="python"
-table = df.groupby(["city", "grade"]).size().unstack()
+table = df.groupby(["city", "grade"]).size().unstack(fill_value=0)
```

## What the rule reports

The rule reports `unstack` right after `size`, `count`, `value_counts` or `nunique` without `fill_value`. A pair without rows comes back as a missing value, and the table shows the dtype of the counts.

| Call | Cell of a pair without rows | Dtype of the counts |
| --- | --- | --- |
| `unstack()` | missing value | `float64` |
| `unstack(fill_value=0)` | 0 | `int64` |

The fix is unsafe, because a mean over the cells changes when the missing values become 0. The rule leaves an `unstack` with more than one argument alone.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `fill_value=0`.
