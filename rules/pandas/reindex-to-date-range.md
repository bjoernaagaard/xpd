Reindexing to a `date_range` from the first to the last time fills in missing steps by hand. Call `asfreq` with the frequency instead.

```diff lang="python"
-full = ts.reindex(pd.date_range(ts.index.min(), ts.index.max(), freq="15min"))
+full = ts.asfreq("15min")
```

## What the rule reports

The rule reports `ts.reindex(pd.date_range(ts.index.min(), ts.index.max(), freq="15min"))`, where `asfreq` fills in the missing steps in one call.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `ts.asfreq("15min")`.

The fix is unsafe, because `asfreq` keeps the name of the index and the reindex dropped it.
