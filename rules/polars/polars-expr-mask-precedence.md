Polars 2.1.0 raises TypeError for this unparenthesized mask. Enclose each comparison in parentheses before combining it.

```diff lang="python"
-expr > 0 & expr < 5
+(expr > 0) & (expr < 5)
```

## What the rule reports

TypeError from chained comparison truth testing.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to parenthesize each comparison around `&`, `|`, or `^`.
