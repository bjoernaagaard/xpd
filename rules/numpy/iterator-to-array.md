`np.array` and NumPy reductions do not read values out of a generator, `map` or a set, so the call gives a wrong value or raises. Pass a list, such as a list comprehension.

```diff lang="python"
-lengths = np.array(len(word) for word in words)
+lengths = np.array([len(word) for word in words])
```

## What the rule reports

The rule reports `np.array(len(w) for w in y)`, `np.array(map(float, y))`, `np.array(d.values())`, `np.array({1, 2})`, `np.max(v for v in y)` and `np.sum(map(f, y))`, because NumPy doesn't read the values out of an iterator. The table shows what each call returned on NumPy 2.5.5.

| Call on a generator | Result |
| --- | --- |
| `np.array(v for v in y)` | an object array with no dimensions that holds the generator |
| `np.max(v for v in y)` | the generator itself |
| `np.any(v for v in [0, 0])` | True |
| `np.sum(v for v in y)` | TypeError "Calling np.sum(generator) is deprecated" |

The rule leaves `np.fromiter` alone, because it reads the values out of an iterator. When the
comprehension is arithmetic on each value, as in `[v * 2 for v in values]`,
[`per-value-comprehension`](per-value-comprehension.md) then reports the list, because the same
arithmetic on the whole array runs faster.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.array([len(w) for w in y])`, and `np.array(sorted({1, 2, 3}))` for a set, where sorted raises on values that do not compare.
