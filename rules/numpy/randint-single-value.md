The upper bound of `randint` is excluded, so `randint(0, 1)` always returns 0. Raise the upper bound by one.

```diff lang="python"
-coin = rng.integers(0, 1, size=10)
+coin = rng.integers(0, 2, size=10)
```

## What the rule reports

The rule reports `np.random.randint(0, 1, n)`, `np.random.randint(1)` and `rng.integers(0, 1)`. Each call always returns 0, because the upper bound is excluded.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.random.randint(0, 2)` and `rng.integers(0, 2)`, with the other arguments kept, so the result is a 0 or a 1 where it was always 0.
