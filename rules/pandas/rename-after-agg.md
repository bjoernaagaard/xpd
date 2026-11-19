Renaming columns right after `agg` names them in a second step. Name them in `agg` itself, as in `agg(b_mean=("b", "mean"))`.

```diff lang="python"
-summary = df.groupby("region").agg({"load": "mean"}).rename(columns={"load": "mean_load"})
+summary = df.groupby("region").agg(mean_load=("load", "mean"))
```

## What the rule reports

The rule reports `g.agg({"b": "mean"}).rename(columns={"b": "b_mean"})`.

## How to fix it

xpd has no automatic fix, so make the change by hand.
