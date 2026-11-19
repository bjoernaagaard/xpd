Building the array of squares to add them up was 3.9 times slower. Call `np.linalg.norm`.

```diff lang="python"
-distance = np.sqrt(np.sum((a - b) ** 2))
+distance = np.linalg.norm(a - b)
```

## What the rule reports

The rule reports `np.sqrt(np.sum(x ** 2))`, `np.sqrt(np.sum(x * x))`, `np.sqrt(np.sum(np.square(x)))`, `np.sqrt((x ** 2).sum())` and `np.sqrt((x * x).sum())`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.linalg.norm(x)`.
