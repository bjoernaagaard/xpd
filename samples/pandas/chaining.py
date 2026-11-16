"""A frame built in separate steps, and the chain that says the same.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import pandas as pd

CASES = {}
RATE = 1.25


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def orders(n=200, seed=0):
    r = np.random.default_rng(seed)
    price = r.normal(100, 15, n).round(2)
    price[::17] = np.nan
    return pd.DataFrame({
        "price": price,
        "qty": r.integers(1, 9, n),
        "city": r.choice(["Oslo", " bergen", "Trondheim "], n),
    })


@case
def lambda_in_assign():
    df = orders()
    return df.assign(
        total=lambda d: d["price"] * d["qty"],  # xpd: lambda-in-assign
        with_tax=lambda d: d["total"] * RATE,  # xpd: lambda-in-assign
        share=lambda d: d["total"] / d["total"].sum(),  # xpd: lambda-in-assign
        town=lambda d: d["city"].str.strip().str.title(),  # xpd: lambda-in-assign
        capped=lambda d: d["price"].where(d["qty"] > 2, 0),  # xpd: lambda-in-assign
        large=lambda d: (d["qty"] > 4) & ~(d["price"] > 110),  # xpd: lambda-in-assign
    )


@case
def lambda_in_a_selection():
    df = orders()
    by_loc = df.loc[lambda d: d["qty"] > 4]  # xpd: lambda-in-assign
    by_brackets = df[lambda d: d["price"] * d["qty"] > 500]  # xpd: lambda-in-assign
    after_a_step = df.rename(columns={"qty": "units"}).loc[lambda d: d["units"] == 1]  # xpd: lambda-in-assign
    return by_loc, by_brackets, after_a_step


@case
def lambdas_that_stay():
    """The frame used as a whole, a column passed to a function, and another frame read by name."""
    df = orders()
    other = orders(seed=1)
    names = ["price", "qty"]
    return df.assign(
        rows=lambda d: len(d),
        by_attribute=lambda d: d.price * 2,
        by_list=lambda d: d[names].sum(axis=1),
        logged=lambda d: np.log(d["price"]),
        against_other=lambda d: d["price"] - other["price"],
        constant=lambda d: 1,
    )


@case
def reassign_ladder():
    df = orders()
    df = df.dropna(subset=["price"])  # xpd: reassign-ladder
    df = df.rename(columns={"qty": "units"})
    df = df.assign(total=pd.col("price") * pd.col("units"))
    df = df.sort_values("total", ignore_index=True)
    chained = (
        orders()
        .dropna(subset=["price"])
        .rename(columns={"qty": "units"})
        .assign(total=pd.col("price") * pd.col("units"))
        .sort_values("total", ignore_index=True)
    )
    return df, df.equals(chained)


@case
def ladders_that_stay():
    """Two steps, a ladder of str methods, and steps with another statement between them."""
    df = orders()
    df = df.dropna(subset=["price"])
    df = df.rename(columns={"qty": "units"})
    name = " Oslo "
    name = name.strip()
    name = name.lower()
    name = name.title()
    other = orders(seed=1)
    other = other.dropna(subset=["price"])
    rows = len(other)
    other = other.rename(columns={"qty": "units"})
    other = other.reset_index(drop=True)
    return df, name, rows, other


def clean(frame):
    return frame.dropna(subset=["price"])


def add_totals(frame):
    return frame.assign(total=pd.col("price") * pd.col("qty"))


def add_shares(frame):
    return frame.assign(share=pd.col("total") / pd.col("total").sum())


def typed(frame: pd.DataFrame):
    return add_shares(add_totals(clean(frame)))  # xpd: nested-step-calls


@case
def nested_step_calls():
    df = pd.DataFrame(orders())
    nested = add_shares(add_totals(clean(df)))  # xpd: nested-step-calls
    from_the_maker = add_shares(add_totals(clean(pd.concat([orders(), orders(seed=1)]))))  # xpd: nested-step-calls
    piped = df.pipe(clean).pipe(add_totals).pipe(add_shares)
    return nested, from_the_maker, typed(df), nested.equals(piped)


@case
def nested_calls_that_stay():
    """Builtins, two steps, and an argument that xpd cannot prove to be a frame."""
    df = pd.DataFrame(orders())
    unknown = orders()
    return len(list(set(df.columns))), add_totals(clean(df)), add_shares(add_totals(clean(unknown)))
