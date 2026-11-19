Dividing a column by its own `shift` computes the change from the earlier row by hand. Call `pct_change()` instead.

```diff lang="python"
-change = df["load"] / df["load"].shift(96) - 1
+change = df["load"].pct_change(96)
```

## What the rule reports

The rule reports `df["load"] / df["load"].shift(96) - 1` and `(df["load"] - df["load"].shift(96)) / df["load"].shift(96)`, which `pct_change` computes in one call.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["load"].pct_change(96)`.

The fix is unsafe, because `(s - s.shift(n)) / s.shift(n)` rounds differently. The result is close to the old one and not equal bit for bit. On pandas 2 `pct_change` also filled missing values forward first.
