`and`, `or`, `not` and chained comparisons ask for the truth of a whole array, which raises ValueError. Combine masks with `&`, `|` and `~` instead.

```diff lang="python"
-inside = x[(x > 2) and (x < 8)]
+inside = x[(x > 2) & (x < 8)]
```

## What the rule reports

The rule reports `x[(x > 2) and (x < 8)]`, `x[not (x > 2)]` and `x[2 < x < 8]`. Each one raises ValueError, because `and`, `or`, `not` and a chained comparison ask for the truth value of a whole array.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `x[(x > 2) & (x < 8)]` for `and`, `|` for `or`, `~` for `not`, and `(2 < x) & (x < 8)` for a chained comparison of names and numbers. On an array of one value the old form indexed with one bool. Other operands, such as a mask held in a name, have no fix.
