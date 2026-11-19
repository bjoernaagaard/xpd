`np.power(x, 0.5)` runs the general power function, which was 2 times slower. Call `np.sqrt`.

```diff lang="python"
-root = np.power(area, 0.5)
+root = np.sqrt(area)
```

## What the rule reports

The rule reports `np.power(x, 0.5)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.sqrt(x)`.
