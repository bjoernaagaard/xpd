"""Trace order amounts to the dashboard with xpd impact."""

import pandas as pd


def totals(orders: pd.DataFrame) -> pd.DataFrame:
    return orders.assign(net_amount=orders["gross_amount"] - orders["discount"])


def shares(orders: pd.DataFrame) -> pd.DataFrame:
    spent = orders.groupby("customer")["net_amount"].transform("sum")
    return orders.assign(customer_share=orders["net_amount"] / spent)


def publish_names(orders: pd.DataFrame) -> pd.DataFrame:
    return orders.rename(columns={"customer_share": "share"})


def features(orders: pd.DataFrame) -> pd.DataFrame:
    return orders.pipe(totals).pipe(shares).pipe(publish_names)


def dashboard(orders: pd.DataFrame, column: str | None = None) -> pd.DataFrame:
    result = features(orders)
    if column is not None:
        return result[[column]]
    return result[["share"]]


def audit(orders: pd.DataFrame) -> pd.DataFrame:
    return features(orders)[["order_id"]]
