On pandas 3, an integer key on a Series is a label, so `value_counts()[0]` and similar lookups raise KeyError unless 0 is one of the labels. Use `.iloc[0]` for the value at a position.

```diff lang="python"
-top = df["region"].value_counts()[0]
-first = df.iloc[0][0]
+top = df["region"].value_counts().iloc[0]
+first = df.iloc[0, 0]
```

## What the rule reports

The rule reports `df["k"].value_counts()[0]`, `df.iloc[i][0]`, `df.dtypes[0]` and a frame reduction such as `df.isna().sum()[0]`, which raise KeyError, because an integer key on a Series is a label on pandas 3. It also reports a negative key, as in `mean()[-1]`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `.iloc[0]`.

The fix is unsafe, because the old form still works on labels that are whole numbers. On such labels `value_counts()[0]` is the count of the value 0. The same holds for `df.dtypes[0]` and a frame reduction such as `df.sum()[0]`, which get `.iloc[0]` too.
