The `level` argument of reductions such as `sum` was removed, so the call raises TypeError on pandas 3. Group by the level instead, as in `df.groupby(level=0).sum()`.

```diff lang="python"
-totals = df["load"].sum(level="region")
+totals = df["load"].groupby(level="region").sum()
```

## What the rule reports

The rule reports `df.sum(level=0)` and the other reductions with `level=`, which raise TypeError. The fix applies when `level=` is the only argument. It is unsafe, because xpd could not run the old form on pandas 3 to compare the two results.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.groupby(level=0).sum()`.
