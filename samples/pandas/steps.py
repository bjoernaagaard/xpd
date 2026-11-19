"""The frame of samples/pandas/script.py, built by step functions that `pipe` joins.

`xpd check` reports nothing here, and `xpd graph samples/pandas/steps.py` reads the steps as a
pipeline.
"""

import numpy as np
import pandas as pd

RATE = 1.25


def orders() -> pd.DataFrame:
    r = np.random.default_rng(0)
    return pd.DataFrame({
        "price": r.normal(100, 15, 200).round(2),
        "qty": r.integers(1, 9, 200),
        "city": r.choice(["Oslo", "Bergen", "Trondheim"], 200),
    })


def keep_sold(frame: pd.DataFrame) -> pd.DataFrame:
    return frame.loc[
        (pd.col("price") > 90) & (pd.col("qty") >= 2) & (pd.col("city") != "Oslo")
    ]


def add_totals(frame: pd.DataFrame) -> pd.DataFrame:
    return frame.assign(
        total=pd.col("price") * pd.col("qty"),
        with_tax=pd.col("total") * RATE,
        large=pd.col("qty") > 4,
        share=pd.col("total") / pd.col("total").sum(),
    )


def add_regions(frame: pd.DataFrame) -> pd.DataFrame:
    regions = pd.DataFrame({"city": ["Bergen", "Trondheim"], "region": ["west", "mid"]})
    return frame.merge(regions, on="city", validate="many_to_one")


def features(frame: pd.DataFrame) -> pd.DataFrame:
    return frame.pipe(keep_sold).pipe(add_totals).pipe(add_regions).reset_index(drop=True)


result = features(orders())
