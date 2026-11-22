Polars 2.1.0 raises TypeError when a Python boolean operator tests a Polars expression or Series. Use parenthesized masks with &, | or ~.

```diff lang="python"
-(expr > 0) and (expr < 5)
+(expr > 0) & (expr < 5)
```

## What the rule reports

TypeError from ambiguous truth value.

## How to fix it

Edit the code using the replacement in the example.
