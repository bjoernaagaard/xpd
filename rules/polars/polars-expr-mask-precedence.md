Polars 2.1.0 raises TypeError for this unparenthesized mask. Enclose each comparison in parentheses before combining it.

```diff lang="python"
-expr > 0 & expr < 5
+(expr > 0) & (expr < 5)
```

## What the rule reports

TypeError from chained comparison truth testing.

## How to fix it

Edit the code using the replacement in the example.
