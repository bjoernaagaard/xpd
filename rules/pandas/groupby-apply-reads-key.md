`groupby.apply` no longer passes the grouping column to the function, so reading it inside the function raises KeyError. Read the group key from the group's `name` instead.

```diff lang="python"
-labels = df.groupby("region").apply(lambda group: group["region"].iloc[0] + ": " + ", ".join(group["site"]))
+labels = df.groupby("region").apply(lambda group: group.name + ": " + ", ".join(group["site"]))
```

## What the rule reports

The rule reports `df.groupby("k").apply(lambda g: g["k"])`, which raises KeyError.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`group.name` exists only inside the function passed to `apply`, and with more than one group key it holds a tuple.
