A comprehension that merges dicts with `**` for every item builds a new dict for each item and is hard to read. Write a loop that builds each item and fills it with `update()`.

```diff lang="python"
 def table(models, catalog):
-    return [{"id": m.id, **owner(m), **limits(catalog, m)} for m in models]
+    rows = []
+    for m in models:
+        row = {"id": m.id}
+        row.update(owner(m))
+        row.update(limits(catalog, m))
+        rows.append(row)
+    return rows
```

## What the rule reports

The rule reports a comprehension or generator expression that merges dicts with `**` for every item.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The comprehension builds a whole row in one expression, and every part of the row has to be a
helper that returns a dict. A reader follows the loop from top to bottom, and a step in it can
grow without the row being rewritten. A comprehension that maps or filters one value per item is
still fine.

`{**d for d in dicts}`, which Python 3.15 added, is not reported.
