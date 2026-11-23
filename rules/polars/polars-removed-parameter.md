Polars 2.0 removed several keyword arguments. Update each call using the replacement for that API.

```diff lang="python"
-pl.read_csv("data.csv", dtypes={"x": pl.Int64})
+pl.read_csv("data.csv", schema_overrides={"x": pl.Int64})
```

## What the rule reports

| Form | Replacement example | Proof on 2.1.0 |
| --- | --- | --- |
| `pl.read_csv("data.csv", dtypes={"x": pl.Int64})` | `pl.read_csv("data.csv", schema_overrides={"x": pl.Int64})` | ArgumentRemovedError |
| `pl.scan_csv("data.csv", missing_utf8_is_empty_string=True)` | `pl.scan_csv("data.csv", empty_string_is_null=False)` | TypeError |
| `pl.scan_parquet("data.parquet", allow_missing_columns=True)` | `pl.scan_parquet("data.parquet", missing_columns="insert")` | ArgumentRemovedError |
| `pl.read_ipc("data.arrow", rechunk=True)` | `pl.read_ipc("data.arrow").rechunk()` | ArgumentRemovedError |
| `df.join(df, on="x", join_nulls=True)` | `df.join(df, on="x", nulls_equal=True)` | ArgumentRemovedError |
| `df.pivot(columns="x", index="y", values="y")` | `df.pivot(on="x", index="y", values="y")` | ArgumentRemovedError |
| `lf.bottom_k(1, by="x", descending=True)` | `lf.bottom_k(1, by="x", reverse=True)` | ArgumentRemovedError |
| `df.rolling(index_column="x", period="1i", by="y")` | `df.rolling(index_column="x", period="1i", group_by="y")` | ArgumentRemovedError |
| `lf.collect(streaming=True)` | `lf.collect(engine="streaming")` | ArgumentRemovedError |
| `pl.collect_all([lf], streaming=True)` | `pl.collect_all([lf], engine="streaming")` | ArgumentRemovedError |
| `lf.explain(tree_format=True)` | `lf.explain(format="tree")` | ArgumentRemovedError |
| `lf.sink_parquet("data.parquet", retries=1)` | `lf.sink_parquet("data.parquet")` | ArgumentRemovedError |
| `df.write_parquet("data.parquet", retries=1)` | `df.write_parquet("data.parquet")` | ArgumentRemovedError |
| `pl.Categorical(ordering="lexical")` | `pl.Categorical()` | TypeError |
| `pl.Array(pl.Int64, width=2)` | `pl.Array(pl.Int64, shape=2)` | ArgumentRemovedError |
| `pl.corr("x", "y", ddof=1)` | `pl.corr("x", "y")` | TypeError |
| `df.glimpse(return_as_string=True)` | `df.glimpse(return_type="string")` | ArgumentRemovedError |
| `df.to_arrow(future=True)` | `df.to_arrow(compat_level=pl.CompatLevel.newest())` | ArgumentRemovedError |
| `lf.collect(predicate_pushdown=False)` | `lf.collect(optimizations=pl.QueryOptFlags(predicate_pushdown=False))` | ArgumentRemovedError |
| `pl.collect_all([lf], predicate_pushdown=False)` | `pl.collect_all([lf], optimizations=pl.QueryOptFlags(predicate_pushdown=False))` | ArgumentRemovedError |
| `df.shift(n=None)` | `df.shift(n=1)` | ComputeError |

## How to fix it

Use `xpd check --fix` for safe removals such as `ordering="lexical"`, and `xpd check --fix --unsafe-fixes` for parameter renames and boolean mappings. Edit forms without a rewrite using their replacement examples.
