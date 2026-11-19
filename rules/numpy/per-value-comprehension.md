A comprehension that applies arithmetic or a math function to each value runs Python once for every value, which was 60 to 470 times slower. Apply the expression to the whole array.

```diff lang="python"
-roots = np.array([math.sqrt(x) for x in values])
+roots = np.sqrt(np.asarray(values))
```

## What the rule reports

The rule reports `np.array([x * 2 + 1 for x in a])`, `np.array([math.sqrt(x) for x in a])`, `np.array([i * 0.5 for i in range(n)])` and `map(np.sqrt, a)`, where the element is arithmetic or a NumPy or math function on the loop variable.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes the element with the loop variable replaced by `np.asarray(a)`, as in `np.asarray(a) * 2 + 1`, or by `np.arange(n)` for a range, and `np.sqrt(np.asarray(a))` for `map`. Python ints do not wrap where int64 does, a division by zero gives inf where Python raised, and `map` returned an iterator. An element with `if`, `and`, `or`, `not`, `min`, `max` or a math function, and a loop over a call, have no fix.
