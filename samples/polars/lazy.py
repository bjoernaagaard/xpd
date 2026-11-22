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

pl.read_parquet("data.parquet").lazy().filter(expr > 1).select("x")  # xpd: polars-eager-read-before-query
lf.collect().lazy()  # xpd: polars-collect-then-lazy
lf.collect().write_parquet("data.parquet")  # xpd: polars-collect-then-write
for _ in range(3):
    lf.collect()  # xpd: polars-collect-in-loop
lf.schema  # xpd: polars-lazy-schema-property
_ = f"{lf.schema=}"  # xpd: polars-lazy-schema-property
lf.columns  # xpd: polars-lazy-schema-property
lf.dtypes  # xpd: polars-lazy-schema-property
lf.width  # xpd: polars-lazy-schema-property
