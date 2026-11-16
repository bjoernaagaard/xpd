"""A few named lines that the reader finds through a legend.

A line that xpd reports ends with a comment that names the rule. The same lines with their
names written on the chart are in file_wide.py. Each case returns the figures the code builds.
"""

import plotly.graph_objects as go
import plotly.io as pio

pio.templates.default = "simple_white"

CASES = {}
YEARS = [2021, 2022, 2023]


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def legends():
    two = go.Figure(data=[go.Scatter(x=YEARS, y=[1, 2, 4], name="north"), go.Scatter(x=YEARS, y=[2, 2, 3], name="south")])  # xpd: legend-instead-of-labels
    first = go.Figure([go.Scatter(x=YEARS, y=[1, 2, 4], name="north", mode="lines"), go.Scatter(x=YEARS, y=[2, 2, 3], name="south", mode="lines")])  # xpd: legend-instead-of-labels
    one = go.Figure(data=[go.Scatter(x=YEARS, y=[1, 2, 4], name="north")])
    unnamed = go.Figure(data=[go.Scatter(x=YEARS, y=[1, 2, 4]), go.Scatter(x=YEARS, y=[2, 2, 3])])
    points = go.Figure(data=[go.Scatter(x=YEARS, y=[1, 2, 4], name="north", mode="markers"), go.Scatter(x=YEARS, y=[2, 2, 3], name="south", mode="markers")])
    traces = [go.Scatter(x=YEARS, y=[1, 2, 4], name="north"), go.Scatter(x=YEARS, y=[2, 2, 3], name="south")]
    from_name = go.Figure(data=traces)
    many = go.Figure(data=[go.Scatter(x=YEARS, y=[i, i + 1, i + 2], name=f"region {i}") for i in range(8)])
    five = go.Figure(data=[
        go.Scatter(x=YEARS, y=[1, 2, 4], name="a"),
        go.Scatter(x=YEARS, y=[2, 2, 3], name="b"),
        go.Scatter(x=YEARS, y=[3, 2, 3], name="c"),
        go.Scatter(x=YEARS, y=[4, 2, 3], name="d"),
        go.Scatter(x=YEARS, y=[5, 2, 3], name="e"),
    ])
    return two, first, one, unnamed, points, from_name, many, five
