A loop that fills an array from one start after another writes the same positions many times. Find the last start for every position with `np.searchsorted` and index the values with it.

```diff lang="python"
-offered = np.zeros_like(line)
-for level, edge in zip(moved, edges, strict=True):
-    offered[level:] = line[edge]
+step = np.searchsorted(moved, np.arange(line.size), side="right") - 1
+offered = np.where(step >= 0, line[edges[np.maximum(step, 0)]], 0.0)
```

## What the rule reports

The rule reports a `for` loop whose only statement assigns to a slice that runs from a name to
the end, as in `out[start:] = value`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Each position ends with the value of the last start at or before it, and `np.searchsorted` finds
that start for every position at once. On NumPy 2.5.5 the two forms were equal on 2,000 random
inputs with sorted starts. The starts have to be sorted and not negative.
