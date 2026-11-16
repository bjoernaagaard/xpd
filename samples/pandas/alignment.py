"""Rows paired by position where the index would pair them, and joins on the index.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import pandas as pd

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def people():
    return pd.DataFrame({"key": ["a", "b", "c", "d"], "n": [1, 2, 3, 4]})


def regions():
    return pd.DataFrame({"key": ["d", "c", "b", "a"], "region": ["D", "C", "B", "A"]}, index=[3, 2, 1, 0])


@case
def assign_drops_alignment():
    df, other = people(), regions()
    df["by_values"] = other["region"].values  # xpd: assign-drops-alignment
    df["by_numpy"] = other["region"].to_numpy()  # xpd: assign-drops-alignment
    df.loc[:, "by_list"] = other["region"].tolist()  # xpd: assign-drops-alignment
    df["by_label"] = other["region"]
    out = df.assign(in_assign=other["region"].values)  # xpd: assign-drops-alignment
    return out


@case
def assignments_that_stay():
    """A column of the same frame has the same index."""
    df = people()
    df["again"] = df["n"].values
    return df.assign(doubled=df["n"].to_numpy() * 2)


@case
def concat_columns():
    left = pd.DataFrame({"x": [1, 2, 3, 4]})
    right = pd.DataFrame({"y": [10, 20, 30, 40]}, index=[0, 1, 2, 9])
    unchecked = pd.concat([left, right], axis=1)  # xpd: concat-columns-unchecked
    by_name = pd.concat([left, right], axis="columns")  # xpd: concat-columns-unchecked
    shared = pd.concat([left, right], axis=1, join="inner")
    stacked = pd.concat([left, left])
    return unchecked, by_name, shared, stacked


@case
def merge_on_both_indexes():
    left = pd.DataFrame({"x": [1, 2, 3, 4]})
    right = pd.DataFrame({"y": [10, 20, 30, 40]}, index=[0, 1, 2, 9])
    inner = left.merge(right, left_index=True, right_index=True)  # xpd: merge-on-both-indexes, merge-without-validate
    outer = left.merge(right, how="outer", left_index=True, right_index=True, validate="1:1")  # xpd: merge-on-both-indexes
    function = pd.merge(left, right, left_index=True, right_index=True)  # xpd: merge-on-both-indexes, merge-without-validate
    named = left.merge(right, left_index=True, right_index=True, suffixes=("_l", "_r"))  # xpd: merge-on-both-indexes, merge-without-validate
    one_side = left.merge(right, left_on="x", right_index=True, validate="1:1")
    return inner, outer, function, named, one_side


def levels():
    index = pd.MultiIndex.from_product([["a", "b"], ["x", "y"], [1, 2]], names=["k1", "k2", "k3"])
    return pd.DataFrame({"v": range(8)}, index=index)


@case
def tuple_slice_on_levels():
    df = levels()
    wide = df["v"].unstack(["k2", "k3"])
    rows = df.loc[("a", slice(None), 1), :]  # xpd: tuple-slice-on-levels
    columns = wide.loc[:, ("x", slice(None))]  # xpd: tuple-slice-on-levels
    bounded = df.loc[(slice("a", "b"), slice(None), 2), :]  # xpd: tuple-slice-on-levels
    written = df.loc[pd.IndexSlice["a", :, 1], :]
    one_label = df.loc[("a", "x", 1), :]
    return rows, columns, bounded, written, one_label


def trades():
    return pd.DataFrame({"t": pd.to_datetime(["2024-01-01 10:00:03", "2024-01-01 10:00:01"]), "qty": [5, 7]})


def quotes():
    return pd.DataFrame({"t": pd.to_datetime(["2024-01-01 10:00:02", "2024-01-01 10:00:00"]), "bid": [99.5, 99.0]})


@case
def merge_asof_unsorted():
    try:
        return pd.merge_asof(trades(), quotes(), on="t")  # xpd: merge-asof-unsorted
    except ValueError as error:
        return str(error)


@case
def merge_asof_sorted():
    return pd.merge_asof(trades().sort_values("t"), quotes().sort_values("t"), on="t")
