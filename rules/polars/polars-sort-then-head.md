For a small number of rows, top_k or bottom_k can avoid a full sort. Sort the small result if its output order matters.

```diff lang="python"
-df.sort("x", descending=True).head(5)
+df.top_k(5, by="x").sort("x", descending=True)
```

## What the rule reports

Top-k was 2.51x and 3.96x faster on shuffled keys, and slower on already sorted keys.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to rewrite `sort(...).head(...)` to `top_k` or `bottom_k` when output order does not matter.
