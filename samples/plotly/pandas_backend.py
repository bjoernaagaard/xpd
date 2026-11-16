"""Figures that pandas draws with plotly.

This file makes plotly the plotting backend of pandas and does not import plotly. xpd reads
`df.plot.bar(...)` here as `px.bar(df, ...)`, when every argument is named. A line that xpd
reports ends with a comment that names the rule. Each case returns the figures the code builds.
"""

import pandas as pd

pd.options.plotting.backend = "plotly"

CASES = {}
YEARS = [2021, 2022, 2023, 2024]


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def sales():
    return pd.DataFrame({
        "year": YEARS * 2,
        "team": ["north"] * 4 + ["south"] * 4,
        "sold": [95, 98, 101, 104, 90, 93, 97, 99],
        "cost": [40.0, 42.5, 41.0, 44.0, 38.0, 39.5, 40.0, 43.0],
    })


@case
def bars_that_start_above_zero():
    df = sales()
    fig = df.plot.bar(x="year", y="sold", color="team", barmode="group")
    fig.update_yaxes(range=[85, 105])  # xpd: bar-axis-without-zero
    chained = df.plot.bar(x="year", y="sold", color="team", barmode="group").update_yaxes(range=[85, 105])  # xpd: bar-axis-without-zero
    return fig, chained


@case
def bars_from_zero():
    df = sales()
    fig = df.plot.bar(x="year", y="sold", color="team", barmode="group")
    fig.update_yaxes(range=[0, 105])
    return fig


@case
def spline_through_the_points():
    df = sales()
    return df.plot.line(x="year", y="sold", color="team", line_shape="spline")  # xpd: legend-instead-of-labels, spline-line


@case
def template_with_a_tint():
    df = sales()
    return df.plot.scatter(x="cost", y="sold", template="ggplot2")  # xpd: default-template


@case
def arguments_by_position():
    """x and y by position are the second and third argument of px.bar, so xpd does not read the call."""
    df = sales()
    return df.plot.line("year", "sold", color="team", line_shape="spline")
