Methods such as `sort()` and `shuffle()` change the array in place and return None, so `x = x.sort()` sets `x` to None. Call the method on its own line.

```diff lang="python"
-values = values.sort()
+values.sort()
```

## What the rule reports

The rule reports `x = x.sort()`, `m = np.fill_diagonal(m, 0)`, `y = rng.shuffle(y)`, `w = w.fill(0)` and `return v.sort()`. The calls change the array in place and return None, so the code gets None in place of the array.

The rule leaves a sort alone that has an argument other than an axis, such as `xp.sort(x)` in the array API, because such a sort returns a value.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `x.sort()`. A result assigned to another name, or returned, has no fix.
