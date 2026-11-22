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

df.join(df, on="x")  # xpd: polars-join-without-validate
df.join_asof(other, on="x", check_sortedness=False)  # xpd: polars-join-asof-unsorted
pl.Series(["open", "closed"], dtype=pl.Categorical)  # xpd: polars-categorical-vs-enum
pl.sql("SELECT * FROM df")  # xpd: polars-sql-frame-globals
assert df.equals(other)  # xpd: polars-assert-frame-equals
