A comprehension that is one of several arguments of a call makes the call hard to read. Give it a name on the line above, and pass the name.

```diff lang="python"
 def points(cache, days):
-    return pd.concat([pd.read_parquet(cache / f"{day}.parquet") for day in days], ignore_index=True)
+    frames = [pd.read_parquet(cache / f"{day}.parquet") for day in days]
+    return pd.concat(frames, ignore_index=True)
```

## What the rule reports

The rule reports a comprehension or generator expression that is one of several arguments of a
call. It leaves a comprehension alone that is the only argument, as in `sum(x.cost for x in items)`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The call holds a loop, and a reader has to find where the loop ends before the other arguments
make sense. The name on the line above says what the loop builds, and the call then fits on one
short line.
