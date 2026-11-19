`mask.astype(int)` turns a mask into 0s and 1s, so indexing with it picks element 0 and element 1 over and over. Index with the mask itself.

```diff lang="python"
-large = y[(y > 15).astype(int)]
+large = y[y > 15]
```

## What the rule reports

The rule reports `y[(y > 15).astype(int)]`, which picks element 0 and element 1 over and over. It also reports `~(y > 15).astype(int)`, which gives -1 and -2.

The rule leaves alone a lookup such as `labels[(score > t).astype(int)]`, where the mask comes from another array. Such a lookup maps each value to one of two labels.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `y[y > 15]`. The `~` form has no fix.
