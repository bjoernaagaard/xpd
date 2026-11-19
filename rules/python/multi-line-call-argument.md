A call that runs over several lines inside another call's arguments is hard to read. Give its result a name on the line above, and pass the name.

```diff lang="python"
-frames.append(
-    pd.DataFrame(
-        {
-            "side": side,
-            "cost": cost,
-            "value": implied(kind, spread - d, markup),
-        }
-    )
-)
+value = implied(kind, spread - d, markup)
+frame = pd.DataFrame({"side": side, "cost": cost, "value": value})
+frames.append(frame)
```

## What the rule reports

The rule reports a call that runs over several lines and is an argument of another call.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Two argument lists are nested, so a reader has to match the brackets to see which argument
belongs to which call, and every line sits two or three levels deep. A name on the line above
says what the inner call builds, and the outer call then fits on one line.
