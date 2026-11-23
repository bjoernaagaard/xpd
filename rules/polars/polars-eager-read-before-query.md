Start this lazy query with a scan to enable projection and predicate pushdown. On 50,000 and 500,000 Parquet rows the scan ran 4.64 and 32.60 times faster.

```diff lang="python"
-pl.read_parquet("data.parquet").lazy().filter(expr > 1).select("x")
+pl.scan_parquet("data.parquet").filter(expr > 1).select("x")
```

## What the rule reports

4.64x and 32.60x with one thread and row group pruning.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to replace `read_*.lazy()` with the matching `scan_*` call.
