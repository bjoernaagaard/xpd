import os

import polars as pl
import polars.selectors as cs
from polars import read_csv as load_csv

df = pl.DataFrame({"x": [1, 2], "y": [3, 4]})
other = pl.DataFrame({"x": [2, 1], "z": [5, 6]})
lf = pl.LazyFrame({"x": [1, 2], "y": [3, 4]})
other_lf = pl.LazyFrame({"x": [2, 1], "z": [5, 6]})
expr = pl.col("x")
series = pl.Series("x", [1, 2])
categorical = pl.Series(["a"]).cast(pl.Categorical)

df.melt()  # xpd: removed-in-polars
df.with_row_count()  # xpd: removed-in-polars
df.approx_n_unique()  # xpd: removed-in-polars
lf.fetch(5)  # xpd: removed-in-polars
lf.profile()  # xpd: removed-in-polars
lf.with_context([])  # xpd: removed-in-polars
df.__dataframe__()  # xpd: removed-in-polars
expr.agg_groups()  # xpd: removed-in-polars
expr.flatten()  # xpd: removed-in-polars
expr.rechunk()  # xpd: removed-in-polars
expr.where(expr > 0)  # xpd: removed-in-polars
expr.shrink_dtype()  # xpd: removed-in-polars
expr.register_plugin()  # xpd: removed-in-polars
pl.Array(pl.Int64, 2).width  # xpd: removed-in-polars
pl.Enum(["a"]).union(pl.Enum(["b"]))  # xpd: removed-in-polars
expr.dt.datetime()  # xpd: removed-in-polars
expr.dt.with_time_unit("ns")  # xpd: removed-in-polars
expr.str.explode()  # xpd: removed-in-polars
categorical.cat.get_categories()  # xpd: removed-in-polars
categorical.cat.is_local()  # xpd: removed-in-polars
categorical.cat.to_local()  # xpd: removed-in-polars
categorical.cat.uses_lexical_ordering()  # xpd: removed-in-polars
pl.groups("x")  # xpd: removed-in-polars
pl.read_csv_batched("data.csv")  # xpd: removed-in-polars
pl.threadpool_size()  # xpd: removed-in-polars
pl.Config.set_auto_structify(True)  # xpd: removed-in-polars
df.group_by("x").count()  # xpd: removed-in-polars
pl.count()  # xpd: removed-in-polars
pl.Enum(["a"]) | pl.Enum(["b"])  # xpd: removed-in-polars
import polars.interchange.dataframe  # xpd: removed-in-polars
from polars.interchange.protocol import DataFrame as InterchangeFrame  # xpd: removed-in-polars
os.environ["POLARS_AUTO_STRUCTIFY"] = "1"  # xpd: removed-in-polars
os.environ["POLARS_STREAMING_CHUNK_SIZE"] = "1"  # xpd: removed-in-polars
pl.read_csv("data.csv", dtypes={"x": pl.Int64})  # xpd: polars-removed-parameter
pl.scan_csv("data.csv", missing_utf8_is_empty_string=True)  # xpd: polars-removed-parameter
pl.scan_parquet("data.parquet", allow_missing_columns=True)  # xpd: polars-removed-parameter
pl.read_ipc("data.arrow", rechunk=True)  # xpd: polars-removed-parameter
df.join(df, on="x", join_nulls=True)  # xpd: polars-removed-parameter
df.pivot(columns="x", index="y", values="y")  # xpd: polars-removed-parameter
lf.bottom_k(1, by="x", descending=True)  # xpd: polars-removed-parameter
df.rolling(index_column="x", period="1i", by="y")  # xpd: polars-removed-parameter
lf.collect(streaming=True)  # xpd: polars-removed-parameter
pl.collect_all([lf], streaming=True)  # xpd: polars-removed-parameter
lf.explain(tree_format=True)  # xpd: polars-removed-parameter
lf.sink_parquet("data.parquet", retries=1)  # xpd: polars-removed-parameter
df.write_parquet("data.parquet", retries=1)  # xpd: polars-removed-parameter
pl.Categorical(ordering="lexical")  # xpd: polars-removed-parameter
pl.Array(pl.Int64, width=2)  # xpd: polars-removed-parameter
pl.corr("x", "y", ddof=1)  # xpd: polars-removed-parameter
df.glimpse(return_as_string=True)  # xpd: polars-removed-parameter
df.to_arrow(future=True)  # xpd: polars-removed-parameter
lf.collect(predicate_pushdown=False)  # xpd: polars-removed-parameter
pl.collect_all([lf], predicate_pushdown=False)  # xpd: polars-removed-parameter
df.shift(n=None)  # xpd: polars-removed-parameter
series.cut([1.5])  # xpd: deprecated-in-polars
pl.concat([df, df.rename({"x": "z", "y": "w"})], how="horizontal", strict=True)  # xpd: deprecated-in-polars
pl.Utf8  # xpd: deprecated-in-polars
pl.read_avro("data.avro")  # xpd: deprecated-in-polars
df.write_avro("data.avro")  # xpd: deprecated-in-polars


load_csv("data.csv", dtypes={"x": pl.Int64})  # xpd: polars-removed-parameter
