Python applies `&` and `|` before comparisons, so `df["a"] > 1 & df["b"] < 3` compares against the wrong value. Put each comparison in parentheses.

```diff lang="python"
-selected = df[df["load"] > 1 & df["wind"] < 3]
+selected = df[(df["load"] > 1) & (df["wind"] < 3)]
```

## What the rule reports

The rule reports `df["a"] > 1 & df["b"] < 3`, where Python applies `&` before the comparisons, so the code compares `df["a"]` with `1 & df["b"]`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `(df["a"] > 1) & (df["b"] < 3)`. The fix closes the comparison before each `&` or `|` and opens it again after, so the result changes on purpose.
