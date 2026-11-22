A function that calls `reset_index` three times or more keeps moving keys between the index and the columns. Keep the keys as columns from the start.

```diff lang="python"
 def summary(rows):
-    group = rows.groupby(KEY, sort=False)
-    keys = group.size().reset_index()[KEY]
-    labels = group[LABELS].first().reset_index(drop=True)
-    return keys.join(labels).reset_index(drop=True)
+    return rows.groupby(KEY, sort=False, as_index=False)[LABELS].first()
```

## What the rule reports

The rule reports a function that calls `reset_index` three times or more, including calls in
nested blocks. It leaves a return of independent tuple results alone at any block depth when
each result resets its index at most once and the function does not reset indexes elsewhere.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Each call moves the same keys between the index and the columns and copies the frame. A reader
has to track after every line whether a key is an index level or a column. With
`as_index=False` the keys stay columns, and `merge` and `sort_values(ignore_index=True)` leave no
index to reset.
