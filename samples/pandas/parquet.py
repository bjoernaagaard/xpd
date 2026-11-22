"""Frames stored as CSV, pickle and HDF, and Parquet files read whole.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import tempfile
from pathlib import Path

import numpy as np
import pandas as pd

CASES = {}
FOLDER = Path(tempfile.mkdtemp())


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame(n=2000, seed=0):
    r = np.random.default_rng(seed)
    return pd.DataFrame({
        "id": np.arange(n),
        "price": r.normal(100, 15, n).round(2),
        "city": r.choice(["Oslo", "Bergen", "Trondheim"], n),
        "sold": pd.Timestamp("2024-01-01") + pd.to_timedelta(r.integers(0, 365 * 86400, n), unit="s"),
        "grade": pd.Categorical(r.choice(["low", "mid", "high"], n), categories=["low", "mid", "high"], ordered=True),
    })


def stored(name="frame.parquet", index=None):
    path = FOLDER / name
    df = frame()
    (df.set_index(index) if index else df).to_parquet(path)
    return path


def csv_file(name):
    """A CSV that another function reads, so this write is no round trip of one function."""
    path = FOLDER / name
    frame().to_csv(path, index=False)
    return path


@case
def csv_round_trip():
    df = frame()
    path = FOLDER / "clean.csv"
    df.to_csv(path, index=False)  # xpd: csv-round-trip
    again = pd.read_csv(path)
    df.to_csv("clean-by-name.csv", index=False)  # xpd: csv-round-trip
    by_name = pd.read_csv("clean-by-name.csv")
    Path("clean-by-name.csv").unlink()
    return again.dtypes.astype(str).to_dict(), len(by_name)


@case
def csv_for_another_program():
    df = frame()
    df.to_csv(FOLDER / "export.csv", index=False)
    return df.to_csv(index=False)[:20]


@case
def pickle_storage():
    df = frame()
    path = FOLDER / "clean.pkl"
    df.to_pickle(path)  # xpd: pickle-or-hdf-storage
    return pd.read_pickle(path)  # xpd: pickle-or-hdf-storage


@case
def hdf_storage():
    df = frame()
    try:
        df[["id", "price"]].to_hdf(FOLDER / "clean.h5", key="frame")  # xpd: pickle-or-hdf-storage
        return pd.read_hdf(FOLDER / "clean.h5", key="frame")  # xpd: pickle-or-hdf-storage
    except ImportError as error:
        return type(error).__name__


@case
def some_columns():
    path = stored()
    two = pd.read_parquet(path)[["price", "id"]]  # xpd: read-parquet-all-columns
    one = pd.read_parquet(path)["city"]  # xpd: read-parquet-all-columns
    with_index = pd.read_parquet(stored("indexed.parquet", index="sold"))[["id", "price"]]  # xpd: read-parquet-all-columns
    return two, one, with_index


@case
def columns_that_stay():
    """A list held in a name, a mask, and a call that already names its columns."""
    path = stored()
    wanted = ["id", "price"]
    by_name = pd.read_parquet(path)[wanted]
    masked = pd.read_parquet(path)[lambda d: d["id"] < 10]  # xpd: lambda-in-assign
    named = pd.read_parquet(path, columns=["id", "price"])[["id"]]
    return by_name, masked, named


@case
def rows_after_the_read():
    path = stored()
    df = pd.read_parquet(path)
    df = df[df["id"] >= 1800]  # xpd: read-parquet-then-filter
    cities = pd.read_parquet(path)
    cities = cities[cities["city"].isin(["Oslo", "Bergen"])]  # xpd: read-parquet-then-filter
    return df, cities


@case
def rows_that_stay():
    """The filter is already passed, and a mask that comes after another step."""
    path = stored()
    early = pd.read_parquet(path, filters=[("id", ">=", 1800)])
    late = pd.read_parquet(path)
    late["double"] = late["price"] * 2
    late = late[late["id"] >= 1800]
    return early, late


@case
def chunks_joined_again():
    path = csv_file("chunks.csv")
    joined = pd.concat(pd.read_csv(path, chunksize=500))  # xpd: chunked-csv-concat
    as_list = pd.concat([part for part in pd.read_csv(path, chunksize=500)], ignore_index=True)  # xpd: chunked-csv-concat, comprehension-among-arguments
    return joined, as_list


@case
def chunks_reduced_first():
    path = csv_file("chunks-reduced.csv")
    parts = []
    for part in pd.read_csv(path, chunksize=500):
        parts.append(part[part["city"] == "Oslo"])
    return pd.concat(parts)
