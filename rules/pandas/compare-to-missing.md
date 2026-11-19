A comparison with a missing value, such as `df["b"] == np.nan`, is never true. Test for missing values with `isna()`.

```diff lang="python"
-missing = df["load"] == np.nan
+missing = df["load"].isna()
```

## What the rule reports

The rule reports `df["b"] == np.nan`, `df["k"].astype(str) == "nan"` and `df[df["b"] == None]`, which are never true. It reports `None` only inside a subscript, because `config["mode"] == None` on a dict is valid Python. When the file shows that the value comes from NumPy, as in `x = np.arange(3.0)` and then `x == np.nan`, the fix writes `pd.isna(x)`, because a NumPy array has no `.isna()`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["b"].isna()`.
