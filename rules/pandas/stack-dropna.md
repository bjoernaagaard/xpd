`stack` no longer takes `dropna`, so the call raises ValueError on pandas 3. Call `stack()` to keep every row, or `stack().dropna(how="all")` to drop rows that are all missing.

```diff lang="python"
-long = wide.stack(dropna=True)
+long = wide.stack().dropna(how="all")
```

## What the rule reports

The rule reports `df.stack(dropna=...)` with either value, which raises ValueError, and `df.stack(future_stack=False)`, which is deprecated. `dropna=` has no fix, because the replacement depends on its value.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is error, or warning for `future_stack=False`. The unsafe fix writes `stack()` for `stack(future_stack=False)`, which keeps the rows that are all missing. `dropna=` has no fix, because the replacement depends on its value.

For `dropna=False`, write `stack()` alone. The new `stack` keeps the rows that are all missing, which the old default dropped.
