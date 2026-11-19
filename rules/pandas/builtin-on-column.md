A Python builtin such as `sum`, `set` or `list` walks a column one value at a time, which was up to 483 times slower. Call the column method instead.

```diff lang="python"
-total = sum(df["load"])
+total = df["load"].sum()
```

## What the rule reports

The rule reports `sum(df["a"])`, `len(set(df["k"]))`, `sum(df["a"] > 50)`, `set(df["k"])`, `Counter(df["k"])`, `list(df["a"])`, `np.array(df["a"].tolist())`, `len(df["k"].unique())` and `set(df["k"].values)`. All but `len(df["k"].unique())` were 2.6 to 483 times slower than the column method, and `len(df["k"].unique())` took as long as `nunique(dropna=False)`. For `set` and `list` the rule reports only a value before `[` that is named like a frame, because both are also how code copies a value of a dict. Even so, the rule reports a dict of lists with such a name, as in `list(frame_data["names"])`. The rule doesn't run on test files.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["a"].sum()`, `df["k"].nunique()`, `df["k"].nunique(dropna=False)` for `len(df["k"].unique())`, `(df["a"] > 50).sum()`, `set(df["k"].unique())`, `df["a"].tolist()`, `df["a"].to_numpy()`, and `df["k"].value_counts()` for `Counter`, which returns a Series sorted by count where `Counter` returned a dict, and leaves out the missing values.

The fix is unsafe, because the column methods skip missing values and the builtins did not.
