Filtering a frame three or more times in a row, each time into itself, leaves a half-filtered frame under the final name after each line. Write the filters as one chain.

```diff lang="python"
-df = df[df["price"] > 0]
-df = df[df["qty"] >= 2]
-df = df.loc[df["city"] != "Oslo"]
+df = (
+    df.loc[lambda d: d["price"] > 0]
+    .loc[lambda d: d["qty"] >= 2]
+    .loc[lambda d: d["city"] != "Oslo"]
+)
```

## What the rule reports

The rule reports the first of three or more statements in a row that each assign one name a selection of its own rows, as in `df = df[df["price"] > 0]` and `df = df.loc[mask]`. Another statement between two of the lines ends the run, and so does a cell marker (`# %%`, `# <codecell>` or `# In[1]:`), so one step in each cell of a file that runs cell by cell is no run. Each selection must be a condition, which is a comparison, conditions joined with `&` or `|`, a negation with `~`, a lambda, or a method that gives True or False for each row, such as `isin` and `notna`. A selection by a string, a list, a slice or a plain name can select columns or rows by position, so it is no part of a run.

A list, a string and a NumPy array are cut the same way, so xpd must know that the name holds a frame where the run starts. That means one of the following:

- the nearest earlier assignment gives the name a frame, such as `pd.read_parquet(...)`, `pd.DataFrame(...)`, a method of a frame that returns a frame, or a selection of rows from a frame
- the name is a parameter annotated `pd.DataFrame`

The chain gave a frame equal to the three statements. In the chain each condition is a lambda, because `df` still names the frame from before the first filter. On pandas 3.0 and later, [`lambda-in-assign`](lambda-in-assign.md) then shortens each lambda to `pd.col`.

## How to fix it

xpd has no automatic fix, so make the change by hand.
