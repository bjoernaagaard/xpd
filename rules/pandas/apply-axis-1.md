`apply(axis=1)` calls a Python function once for every row, which was hundreds of times slower than working on whole columns. Compute on the columns instead, as in `df["a"] + df["b"]`.

```diff lang="python"
-total = df.apply(lambda row: row["load"] + row["wind"], axis=1)
+total = df["load"] + df["wind"]
```

## What the rule reports

The rule reports `df.apply(f, axis=1)`, and `agg` and `transform` with `axis=1` and a lambda or a `"_".join`, which call a function once per row. On 100,000 rows the `agg` and `transform` forms were 327 to 2,735 times slower than whole columns. A named method, as in `agg("sum", axis=1)`, runs on whole columns and is left alone.

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites simple arithmetic lambdas such as `df.apply(lambda r: r["a"] + r["b"], axis=1)` to `df["a"] + df["b"]`. Complex lambdas, `agg`, and `transform` forms remain manual.
