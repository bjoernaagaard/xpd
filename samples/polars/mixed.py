import pandas as pd
import polars as pl
from pandas import DataFrame as PandasFrame
from polars import DataFrame as PolarsFrame


def compare_libraries():
    polars_frame = PolarsFrame({"key": [1, 2], "amount": [4, 5]})
    pandas_frame = PandasFrame({"key": [1, 2], "amount": [4, 5]})
    pandas_lookup = pd.DataFrame({"key": [1, 2], "rate": [2, 3]})
    melted = pandas_frame.melt(var_name="column", value_name="amount")
    counts = pandas_frame.groupby("key").count()
    joined = pandas_frame.merge(pandas_lookup, on="key", validate="1:1")
    accessed = pandas_frame.loc[:, "key"]
    first = pandas_frame.iloc[0]
    pivoted = pandas_frame.pivot(columns="key", values="amount")
    assert pandas_frame.equals(pandas_frame.copy())
    total = polars_frame.select(pl.col("amount").sum())
    polars_frame = pd.DataFrame({"key": [1, 2]})
    reassigned = polars_frame.melt(var_name="column", value_name="amount")
    return melted, counts, joined, accessed, first, pivoted, total, reassigned


def check_polars_annotation(frame: pl.DataFrame):
    return frame.groupby("key")  # xpd: polars-pandas-api-name


def keep_unknown_receiver(frame):
    return frame.groupby("key")
