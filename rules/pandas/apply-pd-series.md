`apply(pd.Series)` builds one Series for every row to split a column of lists or dicts. Build the frame once with `pd.DataFrame(s.tolist(), index=s.index)`.

```diff lang="python"
-parts = df["pair"].apply(pd.Series)
+parts = pd.DataFrame(df["pair"].tolist(), index=df["pair"].index)
```

## What the rule reports

The rule reports `df["pair"].apply(pd.Series)`, which builds one Series per row to split a column of lists or dicts. It reports only a column taken by a text label, because on a frame `apply` works per column.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.DataFrame(df["pair"].tolist(), index=df["pair"].index)`.

The fix is unsafe, because the old form turned whole numbers into floats when a row also held floats, and the fix keeps them whole.
