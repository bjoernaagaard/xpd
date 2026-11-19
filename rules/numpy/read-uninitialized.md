`np.empty` leaves the values unset, so computing with them uses whatever bytes were in memory. Use `np.zeros` or fill the array before you read it.

```diff lang="python"
-total = np.empty(n) + offset
+total = np.zeros(n) + offset
```

## What the rule reports

The rule reports `np.empty(n).sum()` and `np.empty_like(x) + 1`, which compute with whatever bytes were in memory.

The rule leaves `np.empty(n) * np.nan` alone, because it gives NaN everywhere.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.zeros(n)` and `np.zeros_like(a)` with the same arguments, so the code computes on zeros.
