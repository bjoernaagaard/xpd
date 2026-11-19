These names and arguments warn on pandas 3 and will stop working in a later release. Use the replacement that the example and the table show.

```diff lang="python"
-now = pd.Timestamp.utcnow()
+now = pd.Timestamp.now("UTC")
```

## What the rule reports

The rule reports `pd.set_option("mode.copy_on_write", True)`, `copy=False`, `pd.Timestamp.utcnow()`, `pd.Timestamp.utcfromtimestamp(t)`, and the units `"H"` and `"S"` in `pd.Timedelta("1H")`, `pd.to_timedelta(x, unit="H")` and `resample(offset="1H")`. Each form warns on pandas 3.1.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `pd.Timestamp.now("UTC")`, `pd.Timestamp.fromtimestamp(t, "UTC")`, `"h"` and `"s"` for the Timedelta units `"H"` and `"S"`, and it drops `copy=`. The unsafe fix deletes the copy-on-write option line, because `pd.get_option` reads the value back, and drops `copy=` from `astype`.

The rule reports `astype(copy=)` only on a column taken by a label or on a value named like a frame, because NumPy's `astype` has the same keyword. Dropping `copy=` from `astype` is unsafe, because on a NumPy array `astype(copy=False)` can return the same array. The rule deletes the copy-on-write option line at the top level of a file and in a block that holds another statement. As the only statement of a block it reports the line without a fix, because the block would be empty.
