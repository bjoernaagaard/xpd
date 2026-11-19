`kind="mergesort"` is kept only for old code and runs the same stable sort as `kind="stable"`. Write `kind="stable"`.

```diff lang="python"
-order = np.argsort(scores, kind="mergesort")
+order = np.argsort(scores, kind="stable")
```

## What the rule reports

The rule reports `kind="mergesort"` in `np.sort`, `np.argsort` and the `sort` and `argsort` methods. The docs keep `"mergesort"` only for backwards compatibility, and both names select the same stable sort.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `kind="stable"` in `np.sort` and `np.argsort`. The unsafe fix writes the same in a `sort` or `argsort` method, because the pattern cannot tell an array from another object.
