The `in` operator on a column looks among the index labels, not the values, so `2 in df["a"]` can be wrong. Write `(df["a"] == 2).any()`, or use `isin` for many values.

```diff lang="python"
-found = 2 in df["load"]
+found = (df["load"] == 2).any()
```

## What the rule reports

The rule reports `2 in df["a"]`, which looks for 2 among the index labels and not among the values. The rule reports only a value before `[` that is named like a frame, which means `df`, `frame`, or a name with one of them as a word. It leaves `key in data["k"]` alone, because such code is more often a test on a nested dict or a string.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `(df["a"] == 2).any()`.

The fix is unsafe, because xpd also reports `"x" in frame_data["names"]` when `frame_data` holds a dict.
