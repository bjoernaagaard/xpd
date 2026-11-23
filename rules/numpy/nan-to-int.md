Casting an array that holds NaN to an int type turns each NaN into -9223372036854775808. Keep the float array, or replace NaN with a whole number before the cast.

```diff lang="python"
-counts = np.where(seen, totals, np.nan).astype(int)
+counts = np.where(seen, totals, 0).astype(int)
```

## What the rule reports

The rule reports `np.where(m, x, np.nan).astype(int)`, which turns each NaN into -9223372036854775808.

## How to fix it

With `--unsafe-fixes`, xpd removes a bare integer cast to keep the NaNs. If your code requires integers, decide how to represent the missing values.
