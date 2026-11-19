Sorting every value to read the smallest or largest few was up to 39 times slower. Use `np.min`, `np.max`, `np.partition` or `np.argpartition`.

```diff lang="python"
-top = np.sort(scores)[-5:]
+top = np.sort(np.partition(scores, -5)[-5:])
```

## What the rule reports

The rule reports `np.sort(x)[0]`, `np.sort(x)[-1]`, `np.sort(x)[:5]`, `np.sort(x)[-5:]`, `np.sort(x)[::-1][:5]`, `x[np.argsort(x)]`, `np.argsort(x)[0]`, `np.argsort(x)[-1]`, `np.argsort(x)[:5]`, `np.argsort(x)[-5:]` and `np.argsort(np.argsort(x))`, with a literal count under 100.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.min(x)`, `np.max(x)`, `np.sort(np.partition(x, 5)[:5])`, `np.sort(np.partition(x, -5)[-5:])`, `np.sort(x)`, `np.argmin(x)`, `np.argmax(x)`. The positions of the k smallest or largest and the rank by a double argsort have no fix.
