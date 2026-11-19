Nested `np.where` calls read from the inside out and build an array for every level. Use `Series.case_when`, or `pd.cut` for numeric bins.

```diff lang="python"
-band = np.where(df["load"] < 20, "low", np.where(df["load"] < 70, "mid", "high"))
+band = pd.Series("high", index=df.index).case_when([
+    (df["load"] < 20, "low"),
+    (df["load"] < 70, "mid"),
+])
```

## What the rule reports

The rule reports `np.where(a < 20, "low", np.where(a < 70, "mid", "high"))`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`case_when` uses the first condition that matches, and it returns a Series instead of a NumPy array.
