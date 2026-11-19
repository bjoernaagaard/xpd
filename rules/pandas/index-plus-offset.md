Adding an offset to the index by hand replaces the index in a separate step. Call `shift(freq=)`, which moves the index and leaves the values alone.

```diff lang="python"
-ts.index = ts.index + pd.Timedelta(hours=6)
+ts = ts.shift(freq=pd.Timedelta(hours=6))
```

## What the rule reports

The rule reports `ts.index = ts.index + pd.Timedelta(hours=6)` and `ts.index += timedelta(days=1)`, where `shift(freq=)` moves the index in one call.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `ts = ts.shift(freq=pd.Timedelta(hours=6))`.

The fix is unsafe, because the assignment changes the object in place and `shift` returns a new one.
