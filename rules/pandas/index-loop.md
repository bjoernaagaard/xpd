A loop over index labels that reads the frame with `loc` or `at` runs Python once for every row. Compute on whole columns instead.

```diff lang="python"
-out = []
-for label in df.index:
-    out.append(df.loc[label, "load"] * df.loc[label, "weight"])
+out = (df["load"] * df["weight"]).tolist()
```

## What the rule reports

The rule reports `for label in df.index` with `df.loc[label, "a"]` or `df.at[label, "a"]` in the body, also as a comprehension, which reads the frame one label at a time. It leaves a loop alone that never reads the frame by its label, and it doesn't run on test files.

## How to fix it

xpd has no automatic fix, so make the change by hand.
