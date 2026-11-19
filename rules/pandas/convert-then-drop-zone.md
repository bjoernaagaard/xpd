`tz_convert("UTC").tz_localize(None)` takes two steps to get UTC times without a time zone. `tz_convert(None)` does the same in one call.

```diff lang="python"
-plain = df["time"].dt.tz_convert("UTC").dt.tz_localize(None)
+plain = df["time"].dt.tz_convert(None)
```

## What the rule reports

The rule reports `df["d"].dt.tz_convert("UTC").dt.tz_localize(None)`, and the same on an index or a frame, where `tz_convert(None)` gives the same UTC times in one step.

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `df["d"].dt.tz_convert(None)`.
