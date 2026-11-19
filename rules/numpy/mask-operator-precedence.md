Python applies `&` and `|` before comparisons, so `x > 2 & x < 8` compares against the wrong value. Put each comparison in parentheses.

```diff lang="python"
-inside = x[x > 2 & x < 8]
+inside = x[(x > 2) & (x < 8)]
```

## What the rule reports

The rule reports `x > 2 & x < 8`. Python applies `&` before the comparisons, so the code compares `x` with `2 & x`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `(x > 2) & (x < 8)` when the four operands are names, numbers or attributes. Other operands have no fix.

In a file that imports pandas, the pandas rule [`mask-precedence`](../pandas/mask-precedence.md) reports the code instead.
