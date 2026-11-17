"""Panels of make_subplots that show one measure, each on a y scale of its own.

A line that xpd reports ends with a comment that names the rule. The same panels on one
scale are in graph_objects.py. Each case returns the figures the code builds.
"""

import pandas as pd
import plotly.graph_objects as go
import plotly.io as pio
from plotly.subplots import make_subplots

pio.templates.default = "simple_white"

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def sales():
    return pd.DataFrame({"shop": ["north", "north", "south", "south"], "month": [1, 2, 1, 2], "sales": [5, 6, 10, 12], "staff": [3, 3, 8, 9]})


@case
def free_cells():
    df = sales()
    by_shop = make_subplots(rows=1, cols=2)
    for i, (shop, part) in enumerate(df.groupby("shop")):
        by_shop.add_trace(go.Bar(x=part["month"], y=part["sales"], name=shop), row=1, col=i + 1)  # xpd: free-facet-scales
    stacked = make_subplots(rows=2, cols=1, shared_xaxes=True)
    for i, (shop, part) in enumerate(df.groupby("shop")):
        row = i + 1
        stacked.add_trace(go.Scatter(x=part["month"], y=part["sales"], name=shop), row, 1)  # xpd: free-facet-scales
    return by_shop, stacked


@case
def other_cells():
    df = sales()
    measures = make_subplots(rows=1, cols=2)
    for i, column in enumerate(["sales", "staff"]):
        measures.add_trace(go.Bar(x=df["shop"], y=df[column], name=column), row=1, col=i + 1)
    top = 1
    one_cell = make_subplots(rows=1, cols=2)
    for shop, part in df.groupby("shop"):
        one_cell.add_trace(go.Scatter(x=part["month"], y=part["sales"], name=shop), row=top, col=1)
    by_hand = make_subplots(rows=1, cols=2)
    by_hand.add_trace(go.Bar(x=[1, 2], y=[5, 6]), row=1, col=1)
    by_hand.add_trace(go.Bar(x=[1, 2], y=[10, 12]), row=1, col=2)
    # A long table from pd.melt holds a different measure in each panel under one column name.
    long = df.melt(id_vars=["shop", "month"], value_vars=["sales", "staff"])  # xpd: melt-default-names
    melted = make_subplots(rows=1, cols=2)
    for i, (measure, part) in enumerate(long.groupby("variable")):
        melted.add_trace(go.Bar(x=part["month"], y=part["value"], name=measure), row=1, col=i + 1)
    return measures, one_cell, by_hand, melted


@case
def dots():
    # update_traces turns the two named traces into points, so the legend names no lines.
    years = [2021, 2022, 2023]
    dots = go.Figure()
    dots.add_trace(go.Scatter(x=years, y=[1, 2, 4], name="north"))
    dots.add_trace(go.Scatter(x=years, y=[2, 2, 3], name="south"))
    dots.update_traces(mode="markers")
    return dots
