A function that takes columns out with `to_numpy` three times or more and writes arrays back keeps the row order by hand. Compute on the columns in pandas, or take the arrays out once and return arrays.

```diff lang="python"
 def with_share(rows):
-    load = rows["load"].to_numpy()
-    total = rows.groupby("region")["load"].transform("sum").to_numpy()
-    unit = rows["unit_mw"].to_numpy()
-    rows["share"] = load / total * unit
-    return rows
+    total = rows.groupby("region")["load"].transform("sum")
+    return rows.assign(share=rows["load"] / total * rows["unit_mw"])
```

## What the rule reports

The rule reports a function that calls `to_numpy` three times or more and also assigns to a
column of a frame.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The function computes in NumPy what the frame came for, and the arrays stay lined up with the
rows only as long as nothing sorts or filters the frame in between. Columns of one frame are
already lined up, so arithmetic on them needs no array. When the work really is array work, take
the arrays out once and return arrays.
