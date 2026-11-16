"""A rolling window counted in rows on a frame that names its time column.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes.
"""

import pandas as pd

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def days():
    day = pd.to_datetime(["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-10", "2024-01-11", "2024-01-20"])
    return pd.DataFrame({"day": day, "sold": [1, 2, 4, 8, 16, 32]})


@case
def row_window_with_a_time_column():
    df = days()
    by_rows = df.rolling(3, on="day")["sold"].sum()  # xpd: row-window-on-uneven-times
    by_keyword = df.rolling(window=3, on="day", min_periods=1)["sold"].sum()  # xpd: row-window-on-uneven-times
    by_time = df.rolling("3D", on="day")["sold"].sum()
    return by_rows, by_keyword, by_time


@case
def windows_that_stay():
    """A row window without a time column, and a window held in a name."""
    df = days()
    width = 3
    return df["sold"].rolling(3).sum(), df.rolling(width, on="day")["sold"].sum()
