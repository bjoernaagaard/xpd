"""Filled areas whose axis is cut, and sunbursts that draw values as wedges in rings.

A line that xpd reports ends with a comment that names the rule. Each case returns the
figures the code builds.
"""

import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio

pio.templates.default = "simple_white"

CASES = {}
LOW = 90


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def sales():
    return pd.DataFrame({"month": [1, 2, 3, 4], "sales": [95, 97, 99, 100]})


@case
def cut_areas():
    df = sales()
    cut = px.area(df, x="month", y="sales", range_y=[90, 101])  # xpd: area-axis-without-zero
    later = px.area(df, x="month", y="sales")
    later.update_yaxes(range=[90, 101])  # xpd: area-axis-without-zero
    chained = px.area(df, x="month", y="sales").update_layout(yaxis_range=[90.0, 101.0])  # xpd: area-axis-without-zero
    in_dict = px.area(df, x="month", y="sales")
    in_dict.update_layout(yaxis=dict(range=[90, 101]))  # xpd: area-axis-without-zero
    filled = go.Figure(go.Scatter(x=df["month"], y=df["sales"], fill="tozeroy"))
    filled.update_yaxes(range=[90, 101])  # xpd: area-axis-without-zero
    from_zero = px.area(df, x="month", y="sales", range_y=[0, 101])
    by_name = px.area(df, x="month", y="sales", range_y=[LOW, 101])
    line = px.line(df, x="month", y="sales", range_y=[90, 101])
    unfilled = go.Figure(go.Scatter(x=df["month"], y=df["sales"]))
    unfilled.update_yaxes(range=[90, 101])
    rating = px.area(df, x="month", y="sales", range_y=[1, 100])
    return cut, later, chained, in_dict, filled, from_zero, by_name, line, unfilled, rating


@case
def sunbursts():
    df = pd.DataFrame({"region": ["east", "east", "west", "west"], "shop": ["a", "b", "c", "d"], "sales": [30, 10, 15, 45]})
    rings = px.sunburst(df, path=["region", "shop"], values="sales")  # xpd: sunburst-as-pie
    traced = go.Figure(go.Sunburst(labels=["all", "a", "b"], parents=["", "all", "all"], values=[0, 30, 10]))  # xpd: sunburst-as-pie
    bars = px.bar(df.sort_values("sales"), x="sales", y="shop", color="region", orientation="h")
    tree = px.treemap(df, path=["region", "shop"], values="sales")
    return rings, traced, bars, tree


@case
def sunburst_trees():
    tree = go.Figure(go.Sunburst(labels=["all", "x", "y", "x1", "x2", "x3"], parents=["", "all", "all", "x", "x", "x"]))
    counted = px.sunburst(px.data.tips(), path=["day", "time"])  # xpd: sunburst-as-pie
    return tree, counted
