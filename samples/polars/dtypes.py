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

lf.unpivot(on=["x", "y"]).head(2).collect()  # xpd: polars-streaming-order
pl.Series(["2025-01-01"]).cast(pl.Date)  # xpd: polars-removed-cast
pl.Series([1]).cast(pl.Categorical)  # xpd: polars-removed-cast
pl.Series([1]).cast(pl.List(pl.Int64))  # xpd: polars-removed-cast
pl.Series(["a"]).cast(pl.Categorical).cast(pl.Int64)  # xpd: polars-removed-cast
cs.numeric() | pl.col("x")  # xpd: polars-selector-coercion
pl.Series(["1"]) + pl.Series([2])  # xpd: polars-mixed-dtype-arithmetic
expr == None  # xpd: polars-null-comparison
expr != None  # xpd: polars-null-comparison
(expr > 0) and (expr < 5)  # xpd: polars-expr-boolean-op
(expr is None) or False
(expr is not None) and True
expr > 0 & expr < 5  # xpd: polars-expr-mask-precedence
df.select(pl.when(expr > 1).then(expr))  # xpd: polars-when-without-otherwise
pl.DataFrame([[1, 2], [3, 4]], schema=["x", "y"])  # xpd: polars-dataframe-orient
pl.Series([[1], [2, 3]]).list.get(1)  # xpd: polars-list-get-oob
