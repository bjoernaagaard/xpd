A comprehension that runs over more than one line is hard to read. Write a loop, or give the expression for one item a name and a function.

```diff lang="python"
 def stacks(trades):
-    return {
-        str(lot): Stack(group["spread"].to_numpy(), group["mw"].cumsum().to_numpy())
-        for lot, group in trades.groupby("lot", sort=False)
-    }
+    found = {}
+    for lot, group in trades.groupby("lot", sort=False):
+        spread = group["spread"].to_numpy()
+        depth = group["mw"].cumsum().to_numpy()
+        found[str(lot)] = Stack(spread, depth)
+    return found
```

## What the rule reports

The rule reports a comprehension or generator expression that runs over more than one line.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A comprehension that needs several lines holds more than one step, and its result stands before
the loop that makes it, so a reader goes through it twice. The loop gives each step its own line
and its own name, in the order it runs.
