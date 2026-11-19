A comparison inside `apply` runs Python once for every value. Compare the whole column instead.

```diff lang="python"
-high = df["load"].apply(lambda value: value > 50)
+high = df["load"] > 50
```

## What the rule reports

The rule reports `df["a"].apply(lambda v: v > 50)` and `df["a"].apply(lambda v: v in wanted)`, which compare one value at a time. It leaves a chain such as `0 < v < 5` alone, and a test on text such as `"ta" in v` or `v in "abc"`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["a"] > 50`, `df["a"].isin(wanted)`.

The `isin` fix is unsafe, because a dict, a range or a custom container can answer `in` differently from `isin`.
