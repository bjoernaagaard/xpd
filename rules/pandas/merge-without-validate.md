A merge without `validate=` duplicates rows without a warning when a key repeats. Pass `validate=` with the relation you expect, such as `"many_to_one"`.

```diff lang="python"
-joined = left.merge(right, on="key")
+joined = left.merge(right, on="key", validate="many_to_one")
```

## What the rule reports

The rule reports `merge` without `validate=`, which duplicates rows without a warning when a key repeats. The rule leaves a merge with `how="cross"` alone, because it pairs every row on purpose, and a merge with `how="left_anti"` or `how="right_anti"`, because an anti join keeps each row of one frame at most once.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Pick the `validate` value that matches your data, because `"many_to_one"` is only the example. `"many_to_many"` checks nothing.
