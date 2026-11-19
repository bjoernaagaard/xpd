Querying a KD tree once for every point makes one call for each point. Pass all points to `query` in one call, which was 16 times faster.

```diff lang="python"
 def nearest_points(points, queries):
     tree = KDTree(points)
-    return [tree.query(q)[1] for q in queries]
+    return tree.query(queries)[1]
```

## What the rule reports

The rule reports `tree.query(q)` inside a comprehension or a `for` loop, where `q` is the loop
name or a row indexed by it. `tree` must come from a `KDTree` or `cKDTree` call in the same
function. For 1,000,000 points on a tree of 10,000 points, one query of all points ran 16 to 17
times faster.

## How to fix it

xpd has no automatic fix, so make the change by hand.
