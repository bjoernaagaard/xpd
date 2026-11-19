`reset_index()` after a groupby moves the keys back into columns in a second step. Pass `as_index=False` to `groupby` instead.

```diff lang="python"
-totals = df.groupby("region")["load"].sum().reset_index()
+totals = df.groupby("region", as_index=False)["load"].sum()
```

## What the rule reports

The rule reports `df.groupby("k")[["a", "b"]].sum().reset_index()`. The fix is unsafe, because a key that is not a column comes back in another place.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.groupby("k", as_index=False)[["a", "b"]].sum()`.
