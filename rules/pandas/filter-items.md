`filter(items=)` skips a missing name without an error, and pandas 3.1 plans to deprecate it. Select with `df[names]`, which raises on a missing name.

```diff lang="python"
-selected = df.filter(items=output_columns)
+selected = df[output_columns]
```

## What the rule reports

The rule reports `df.filter(items=cols)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df[cols]`.

The fix is unsafe, because `df[cols]` raises KeyError on a label that is absent, where `filter` skipped the label.
