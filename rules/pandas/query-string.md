A `query` string hides column names from editors, type checkers and xpd. Write the condition as an expression, such as `df.loc[pd.col("a") > 50]`.

```diff lang="python"
-selected = df.query("load > 50 and wind < 3")
+selected = df.loc[(pd.col("load") > 50) & (pd.col("wind") < 3)]
```

## What the rule reports

The rule reports `df.query("a > 50 and b < 0")`, because editors, type checkers and xpd can read the column names in `pd.col("a")` and not in a string. The rule leaves a string that reads like SQL alone, such as `conn.query("SELECT ...")`.

## How to fix it

xpd has no automatic fix, so make the change by hand.
