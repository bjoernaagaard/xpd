`inplace=True` gives no speedup and keeps the call out of a method chain. Assign the result instead.

```diff lang="python"
-df.dropna(inplace=True)
+df = df.dropna()
```

## What the rule reports

The rule reports any other `inplace=True`, which gives no speedup and keeps the call out of a method chain.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df = df.dropna()`.

The fix applies only to a call that stands alone as a statement, on a name, an attribute or a subscript. It drops comments between the arguments of the call.
