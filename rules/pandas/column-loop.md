A loop that reduces one column per pass makes a separate call for every column. Select the columns at once and reduce them in one call, which returns one value per column.

```diff lang="python"
-rows = []
-for name in models:
-    error = (table[name] - table["published"]).dropna()
-    rows.append((name, len(error), error.abs().mean(), error.mean()))
+error = table[models].sub(table["published"], axis=0)
+scores = pd.DataFrame({"n": error.count(), "mae": error.abs().mean(), "bias": error.mean()})
```

## What the rule reports

The rule reports a loop or a comprehension over column names that reduces one column per pass,
or that does arithmetic between that column and another column of the same frame. It also
reports a loop that builds the column name from the loop variable in an f-string.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A frame takes a list of names and reduces every column in one call, and the result is a table
with one row per column. On pandas 3.1.1 the two forms gave the same count, mean absolute error,
mean and root mean square, within 1e-16. For names built in an f-string, build the list of names
once, as in `raw[[f"{ic} Volume" for ic in ZONE]].fillna(0.0)`.
