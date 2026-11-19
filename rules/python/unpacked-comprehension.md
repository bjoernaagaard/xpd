A dict literal that unpacks a comprehension with `**` is hard to read. Build the dict first and add the computed keys in a loop, or give the comprehension a name.

```diff lang="python"
 def row(model, limits):
-    return {"id": model.id, **{f"limit_{k}": v for k, v in limits.items()}}
+    out = {"id": model.id}
+    for name, value in limits.items():
+        out[f"limit_{name}"] = value
+    return out
```

## What the rule reports

The rule reports a dict literal that unpacks a dict comprehension with `**`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The literal builds a dict, unpacks it and merges it in one expression. A reader has to work
through the comprehension to know which keys the result has.
