Under copy-on-write, the array behind a column is read-only, so writing through `.values` raises ValueError. Assign with `.loc` instead.

```diff lang="python"
-df["load"].values[0] = value
+df.loc[df.index[0], "load"] = value
```

## What the rule reports

The rule reports `df["a"].values[0] = 9`, which raises ValueError, because the array is read-only under copy-on-write.

## How to fix it

xpd has no automatic fix, so make the change by hand.
