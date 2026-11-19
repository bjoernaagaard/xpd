Under copy-on-write, an `inplace=True` call on a selected column changes a copy, so the frame keeps its old values. Assign the result back to the column instead.

```diff lang="python"
-df["load"].fillna(0, inplace=True)
+df["load"] = df["load"].fillna(0)
```

## What the rule reports

The rule reports `df["b"].fillna(0, inplace=True)`, which changes a copy, so the frame keeps its old values. It also reports `df[column].fillna(0, inplace=True)`, where a variable names the column.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["b"] = df["b"].fillna(0)`.

The fix is unsafe, because xpd cannot tell a column from a value in a dict. The fix drops comments between the arguments of the call.
