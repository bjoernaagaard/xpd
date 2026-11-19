Sorting a whole frame to keep its first or last few rows does more work than needed. Use `nsmallest` or `nlargest`.

```diff lang="python"
-largest = df.sort_values("load", ascending=False).head(10)
+largest = df.nlargest(10, "load")
```

## What the rule reports

The rule reports `df.sort_values("b").head(10)`, `df.sort_values("b").iloc[0]`, `df["b"].sort_values().iloc[0]`, `df.sort_values("b").tail(10)` and `sorted(df["b"])[0]`, which sort every row to keep a few. It leaves a negative count alone, because `head(-3)` keeps all but the last 3 rows and `nsmallest(-3)` returns no rows.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.nsmallest(10, "b")`, `df.nlargest(10, "b")` for `tail(10)`, `df.iloc[df["b"].argmin()]`, `df.iloc[df["b"].argmax()]` for `iloc[-1]`, `df["b"].min()`, `df["b"].max()`, `df["b"].idxmin()`, and `df["b"].min()` and `.max()` for `sorted(df["b"])[0]` and `[-1]`. A sort by more than one column, a frame that is not held in a name and a negative count of `tail` have no fix.

The fix is unsafe, and the table gives the reason for each form.

| Fix | What can differ |
| --- | --- |
| `nsmallest` and `nlargest` | they raise on a text column, and they order ties differently |
| `df.iloc[df["b"].argmin()]` | `argmin` raises when every value is missing, where the sort returned the first row |
| `idxmin` | it can name another row when the lowest value appears twice |
| `max()` for `sort_values().iloc[-1]` | the sort puts missing values last, so the old form returned NaN where `max()` skips them |
| `nlargest` for `tail` | it lists the rows largest first, where `tail` listed them largest last |
| `min()` and `max()` for `sorted` | they return NumPy scalars where `sorted` gave Python numbers, and `sorted` does not order a column with NaN |
