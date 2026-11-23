Renaming columns right after `agg` names them in a second step. Name them in `agg` itself, as in `agg(b_mean=("b", "mean"))`.

```diff lang="python"
-summary = df.groupby("region").agg({"load": "mean"}).rename(columns={"load": "mean_load"})
+summary = df.groupby("region").agg(mean_load=("load", "mean"))
```

## What the rule reports

The rule reports `g.agg({"b": "mean"}).rename(columns={"b": "b_mean"})`.

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites one- and two-column literal `.agg({...}).rename(columns={...})` calls with valid identifier names to named aggregations (`agg(b_mean=("b", "mean"))`). Larger or dynamic mappings remain manual.
