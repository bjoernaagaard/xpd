"""A method chain that xpd graph reads as a pipeline of steps.

Each .assign(...) of the chain is a step. `xpd graph samples/graph/chain.py` lists which step
reads the columns of which other step, and the columns the chain needs from the file it reads.
"""

import pandas as pd

LIMIT = 1000


def features(orders: pd.DataFrame) -> pd.DataFrame:
    return (
        orders.assign(gross_amount=pd.col("quantity") * pd.col("unit_price"))
        .assign(
            net_amount=lambda d: d["gross_amount"] - d["discount"],  # xpd: lambda-in-assign
            before_discount=lambda d: d["unit_price"] * d["quantity"],  # xpd: lambda-in-assign
        )
        .assign(customer_share=orders["net_amount"] / orders["net_amount"].sum())
        .assign(is_large=pd.col("net_amount") > LIMIT)
    )


def load(path: str) -> pd.DataFrame:  # xpd: pass-through-function
    return pd.read_parquet(path)


margins = (
    load("orders.parquet")
    .assign(unit_margin=pd.col("unit_price") - pd.col("unit_cost"))
    .assign(margin=lambda d: d["unit_margin"] * d["quantity"])  # xpd: lambda-in-assign
)
