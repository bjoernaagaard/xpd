Selecting rows by `index.time` or `index.hour` builds a Python time or an hour for every row. Use `at_time` or `between_time` instead.

```diff lang="python"
-noon = ts[ts.index.time == time(12)]
-working = ts[(ts.index.hour >= 8) & (ts.index.hour < 16)]
+noon = ts.at_time(time(12))
+working = ts.between_time("8:00", "16:00", inclusive="left")
```

## What the rule reports

The rule reports `ts[ts.index.time == time(12)]` and `ts[(ts.index.hour >= 8) & (ts.index.hour < 16)]`, which build a Python time or an hour for every row. The table shows the times on 2,000,000 rows.

| Mask | Time | Replacement | Time |
| --- | --- | --- | --- |
| `ts.index.time == time(12)` | 241 ms | `at_time` | 25 ms |
| the two hour masks | 51 ms | `between_time` | 28 ms |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `ts.at_time(time(12))`. The unsafe fix writes `ts.between_time("8:00", "16:00", inclusive="left")`.

The `at_time` fix applies to a time object, such as `time(12)`. The rule leaves a comparison with text alone, because text never equals a time. The `between_time` fix applies to whole hours written as numbers. It is unsafe when the first hour is later than the second. Then the masks select nothing, and `between_time` selects the hours across midnight.
