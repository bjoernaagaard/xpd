A loop over zipped columns runs Python once for every row. Compute on whole columns instead.

```diff lang="python"
-out = [load * weight for load, weight in zip(df["load"], df["weight"])]
+out = (df["load"] * df["weight"]).tolist()
```

## What the rule reports

The rule reports a loop or comprehension over `zip(df["a"], df["w"])`, which works on one pair of values at a time. It reports only when every argument of `zip` is a column taken by a text label, or a `.values`, `.to_numpy()` or `.tolist()`. It leaves `dict(zip(...))` alone, because it isn't a loop. The rule doesn't run on test files.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Keep the result as a Series when the next step takes one, and call `tolist` only where the next step needs a list.
