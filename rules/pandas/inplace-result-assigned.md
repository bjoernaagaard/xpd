A call with `inplace=True` returns None, so `df = df.dropna(inplace=True)` sets `df` to None. Drop `inplace=True` and keep the assignment.

```diff lang="python"
-df = df.dropna(inplace=True)
+df = df.dropna()
```

## What the rule reports

The rule reports `df = df.dropna(inplace=True)`, which sets `df` to None.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df = df.dropna()`.

The fix drops comments between the arguments of the call.
