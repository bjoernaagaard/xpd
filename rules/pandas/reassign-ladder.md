Assigning a frame three or more times in a row from its own methods leaves a half-built frame under the final name after each line. Write the steps as one method chain.

```diff lang="python"
-df = df.dropna(subset=["price"])
-df = df.rename(columns={"qty": "units"})
-df = df.assign(total=pd.col("price") * pd.col("units"))
+df = (
+    df.dropna(subset=["price"])
+    .rename(columns={"qty": "units"})
+    .assign(total=pd.col("price") * pd.col("units"))
+)
```

## What the rule reports

The rule reports the first of three or more statements in a row that each assign one name from a method of itself, as in `df = df.dropna()`. The rule counts only methods that return a frame or a Series, such as `assign`, `rename`, `dropna`, `merge` and `sort_values`, so it leaves a run of `str` methods alone. Another statement between two of the lines ends the run, and so does a cell marker (`# %%`, `# <codecell>` or `# In[1]:`), so one step in each cell of a file that runs cell by cell is no run. The chain gave a frame equal to the three statements.

## How to fix it

xpd has no automatic fix, so make the change by hand.
