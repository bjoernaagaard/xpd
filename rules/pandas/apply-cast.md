Casting with `apply(int)` or `map(float)` converts one value at a time in Python. Cast the whole column with `astype`.

```diff lang="python"
-whole = df["load"].apply(int)
+whole = df["load"].astype(int)
```

## What the rule reports

The rule reports `df["b"].apply(int)`, `df["raw"].map(float)` and `df["b"].apply(lambda v: int(v))`, which cast one value at a time. With a bare function name it reports `apply` only on a column taken by a text label. The rule covers `int` and `float` only, because on floats with missing values `astype(str)` was slower than `apply(str)` and kept the missing values.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["b"].astype(int)`.
