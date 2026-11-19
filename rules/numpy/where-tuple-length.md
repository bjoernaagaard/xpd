`np.where` with one argument returns a tuple with one array per dimension, so its length counts dimensions and not matches. Use `np.count_nonzero`.

```diff lang="python"
-matches = len(np.where(z > 0))
+matches = np.count_nonzero(z > 0)
```

## What the rule reports

The rule reports `len(np.where(z > 0))`, `len(np.nonzero(z))` and `if np.where(z > 9):`, because `np.where` with one argument returns a tuple with one array per dimension. The length of the tuple is the number of dimensions, and the tuple is always true.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.count_nonzero(z > 0)`.
