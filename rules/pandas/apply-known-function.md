Passing a function such as `len`, `pd.isna` or `pd.to_datetime` to `apply` calls it once for every value. Call the column method or pass the whole column to the function.

```diff lang="python"
-lengths = df["name"].apply(len)
+lengths = df["name"].str.len()
```

## What the rule reports

The rule reports `df["s"].apply(len)`, `df["b"].apply(pd.isna)`, `df["raw"].apply(pd.to_datetime)` and `df["b"].apply(lambda v: round(v, 2))`. It reports only a column taken by a text label, because on a frame `apply` works per column.

Each fix is unsafe, and the table gives the reason.

| Fix | What can differ |
| --- | --- |
| `pd.to_datetime(df["raw"])` | a call per value parses each text on its own, and the whole column must share one format |
| `df["b"].isna()` | on a column of lists, `pd.isna` per value returned an array per cell |
| `df["b"].round(2)` | Python and NumPy round some halfway values differently |
| `df["s"].str.len()` | it raises on a column that is not text |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["s"].str.len()`, which raises on a column that is not text, `df["b"].isna()`, `pd.to_datetime(df["raw"])`, `df["b"].round(2)`.
