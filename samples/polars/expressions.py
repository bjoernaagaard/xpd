import os

import polars as pl
import polars.selectors as cs
from polars.testing import assert_frame_equal

df = pl.DataFrame({"x": [1, 2], "y": [3, 4]})
other = pl.DataFrame({"x": [2, 1], "z": [5, 6]})
lf = pl.LazyFrame({"x": [1, 2], "y": [3, 4]})
other_lf = pl.LazyFrame({"x": [2, 1], "z": [5, 6]})
expr = pl.col("x")
series = pl.Series("x", [1, 2])
categorical = pl.Series(["a"]).cast(pl.Categorical)

expr.map_elements(lambda value: value + 1, return_dtype=pl.Int64)  # xpd: polars-map-elements-native
expr.map_elements(lambda value: (value,))  # xpd: polars-map-without-return-dtype
sum(row[0] for row in df.iter_rows())  # xpd: polars-row-iteration
df.with_columns(a=expr + 1).with_columns(b=expr + 2)  # xpd: polars-sequential-with-columns
output = pl.DataFrame(schema=df.schema)
for piece in df.iter_slices(1):
    output = pl.concat([output, piece], rechunk=True)  # xpd: polars-concat-in-loop
df.join(df.group_by("y").agg(total=expr.sum()), on="y")  # xpd: polars-groupby-join-over,polars-join-without-validate
df.sort("x", descending=True).head(5)  # xpd: polars-sort-then-head
df.groupby("x")  # xpd: polars-pandas-api-name
df.fillna(0)  # xpd: polars-pandas-api-name
df.dropna()  # xpd: polars-pandas-api-name
df.sort_values("x")  # xpd: polars-pandas-api-name
pl.from_pandas(df.to_pandas())  # xpd: polars-pandas-round-trip
len(expr)  # xpd: polars-builtin-on-expr-or-series
