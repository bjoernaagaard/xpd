"""Two rules that read the whole file before they report.

`day-frequency-with-time-zone` reports only in a file that names a time zone, and this file names
none. `datetime-unit-check` leaves a file alone that casts to nanoseconds itself, as `stamps`
does. So xpd reports nothing here.
"""

import numpy as np
import pandas as pd

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame(n=2000, seed=0):
    r = np.random.default_rng(seed)
    return pd.DataFrame({
        "b": r.normal(size=n).round(4),
        "d": pd.Timestamp("2022-01-01") + pd.to_timedelta(np.arange(n) * 15, unit="min"),
    })


def stamps(df):
    return df["d"].astype("datetime64[ns]")


@case
def day_frequency_without_time_zone(df):
    ts = df.set_index("d")
    return (
        ts.shift(freq="D"),
        ts.shift(1, "2D"),
        ts["b"].pct_change(freq="D"),
        ts.index + pd.offsets.Day(1),
        ts.resample("D", offset="6h").sum(),
        ts.groupby(pd.Grouper(freq="D", origin="start")).sum(),
    )


@case
def nanosecond_check_after_cast(df):
    return stamps(df).dtype == "datetime64[ns]", df.assign(d=stamps(df)).select_dtypes("datetime64[ns]")
