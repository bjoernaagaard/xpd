Polars 2.0 removed methods and attributes that previously worked. Use the replacement named for the removed API.

```diff lang="python"
-df.melt()
+df.unpivot()
```

## What the rule reports

| Form | Replacement example | Proof on 2.1.0 |
| --- | --- | --- |
| `df.melt()` | `df.unpivot()` | AttributeRemovedError |
| `df.with_row_count()` | `df.with_row_index(name="row_nr")` | AttributeRemovedError |
| `df.approx_n_unique()` | `df.select(pl.all().approx_n_unique())` | AttributeRemovedError |
| `lf.fetch(5)` | `lf.head(5).collect()` | AttributeRemovedError |
| `lf.profile()` | `lf.explain()` | AttributeRemovedError |
| `lf.with_context([])` | `pl.concat([lf, other_lf], how="horizontal")` | AttributeRemovedError |
| `df.__dataframe__()` | `df.to_arrow()` | AttributeRemovedError |
| `expr.agg_groups()` | `df.with_row_index().group_by("y").agg(pl.col("index"))` | AttributeRemovedError |
| `expr.flatten()` | `expr.list.explode(keep_nulls=False, empty_as_null=False)` | AttributeRemovedError |
| `expr.rechunk()` | `lf.collect().rechunk()` | AttributeRemovedError |
| `expr.where(expr > 0)` | `expr.filter(expr > 0)` | AttributeRemovedError |
| `expr.shrink_dtype()` | `lf.collect()["x"].shrink_dtype()` | AttributeRemovedError |
| `expr.register_plugin()` | `from polars.plugins import register_plugin_function` | AttributeRemovedError |
| `pl.Array(pl.Int64, 2).width` | `pl.Array(pl.Int64, 2).size` | AttributeRemovedError |
| `pl.Enum(["a"]).union(pl.Enum(["b"]))` | `pl.Enum(["a", "b"])` | AttributeRemovedError |
| `expr.dt.datetime()` | `expr.dt.replace_time_zone(None)` | AttributeRemovedError |
| `expr.dt.with_time_unit("ns")` | `expr.cast(pl.Int64).cast(pl.Datetime('ns'))` | AttributeRemovedError |
| `expr.str.explode()` | `expr.str.split("").explode()` | AttributeError |
| `categorical.cat.get_categories()` | `categorical.unique()` | AttributeRemovedError |
| `categorical.cat.is_local()` | `categorical.dtype` | AttributeRemovedError |
| `categorical.cat.to_local()` | `categorical` | AttributeRemovedError |
| `categorical.cat.uses_lexical_ordering()` | `categorical.sort()` | AttributeRemovedError |
| `pl.groups("x")` | `df.with_row_index().group_by("y").agg(pl.col("index"))` | AttributeRemovedError |
| `pl.read_csv_batched("data.csv")` | `pl.scan_csv("data.csv").collect_batches()` | AttributeRemovedError |
| `pl.threadpool_size()` | `pl.thread_pool_size()` | AttributeRemovedError |
| `pl.Config.set_auto_structify(True)` | `df.select(pl.struct("x", "y"))` | AttributeRemovedError |
| `df.group_by("x").count()` | `df.group_by("x").len(name="count")` | AttributeRemovedError |
| `pl.count()` | `pl.len()` | TypeError with 0 arguments |
| `pl.Enum(["a"]) \| pl.Enum(["b"])` | `pl.Enum(["a", "b"])` | TypeError |
| `import polars.interchange.dataframe` | `from polars import CompatLevel` | ModuleNotFoundError |
| `from polars.interchange.protocol import DataFrame as InterchangeFrame` | `from polars import CompatLevel` | All 11 removed protocol imports raised ImportError. |
| `os.environ["POLARS_AUTO_STRUCTIFY"] = "1"` | `df.select(pl.struct("x", "y"))` | Config.state omitted the variable when it was set to 1. |
| `os.environ["POLARS_STREAMING_CHUNK_SIZE"] = "1"` | `os.environ["POLARS_IDEAL_MORSEL_SIZE"] = "1"` | Config.state omitted the variable when it was set to 1. |

## How to fix it

Use `xpd check --fix --unsafe-fixes` to apply the offered rewrite after checking the intended result.
Edit forms without a rewrite using their replacement examples.
