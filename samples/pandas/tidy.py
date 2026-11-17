"""A wide frame made long and back, the longer way and with defaults that change values.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import pandas as pd

CASES = {}
YEARS = ["2022", "2023", "2024"]


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def wide(n=50, seed=0):
    r = np.random.default_rng(seed)
    return pd.DataFrame({"id": np.arange(n), "city": r.choice(["Oslo", "Bergen"], n)} | {year: r.integers(0, 90, n) for year in YEARS})


@case
def column_loop_to_long():
    df = wide()
    in_a_list = pd.concat([df[["id", year]].rename(columns={year: "sales"}).assign(year=year) for year in YEARS], ignore_index=True)  # xpd: column-loop-to-long
    named = [df[["id", year]].rename(columns={year: "sales"}).assign(year=year) for year in YEARS]  # xpd: column-loop-to-long
    from_name = pd.concat(named, ignore_index=True)
    parts = []
    for year in YEARS:  # xpd: column-loop-to-long
        part = df[["id", "city", year]].rename(columns={year: "sales"})
        parts.append(part.assign(year=year))
    appended = pd.concat(parts, ignore_index=True)
    melted = df.melt(id_vars=["id", "city"], value_vars=YEARS, var_name="year", value_name="sales")
    return in_a_list, from_name, appended, appended[melted.columns].equals(melted)


@case
def loops_that_stay():
    """One frame per file, and a loop that renames nothing."""
    frames = [wide(seed=seed).assign(run=seed) for seed in range(3)]
    stacked = pd.concat([frame[["id", "city"]] for frame in frames])
    totals = []
    for year in YEARS:
        totals.append(wide()[year].sum())
    return stacked, totals


@case
def stack_then_reset_index():
    df = wide()
    long = df.set_index(["id", "city"]).stack().reset_index()  # xpd: stack-then-reset-index
    series = df.set_index("id")[YEARS].stack()
    return long, series


@case
def melt_default_names():
    df = wide()
    default = df.melt(id_vars=["id", "city"])  # xpd: melt-default-names
    function = pd.melt(df, id_vars=["id", "city"], value_name="sales")  # xpd: melt-default-names
    named = df.melt(id_vars=["id", "city"], var_name="year", value_name="sales")
    return default, function, named


@case
def pivot_table_default_mean():
    df = wide().melt(id_vars=["id", "city"], var_name="year", value_name="sales")
    mean = df.pivot_table(index="city", columns="year", values="sales")  # xpd: pivot-table-default-mean
    function = pd.pivot_table(df, index="city", columns="year", values="sales")  # xpd: pivot-table-default-mean
    stated = df.pivot_table(index="city", columns="year", values="sales", aggfunc="sum")
    one_row_per_cell = df.pivot(index="id", columns="year", values="sales")
    return mean, function, stated, one_row_per_cell


@case
def unstack_without_fill():
    df = pd.DataFrame({"city": ["Oslo", "Oslo", "Bergen"], "grade": ["low", "high", "low"], "n": [1, 2, 3]})
    sizes = df.groupby(["city", "grade"]).size().unstack()  # xpd: unstack-without-fill
    counts = df.groupby("city")["grade"].value_counts().unstack("grade")  # xpd: unstack-without-fill
    filled = df.groupby(["city", "grade"]).size().unstack(fill_value=0)
    sums = df.groupby(["city", "grade"])["n"].sum().unstack()
    return sizes, counts, filled, sums
