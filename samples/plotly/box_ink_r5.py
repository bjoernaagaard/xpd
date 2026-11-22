"""Box outlines and the conditional quartile drawing for a five-number summary."""

import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio
from plotly import express as express
from plotly import graph_objects as graphs

pio.templates.default = "simple_white"
CASES = {}
VALUES = [1, 2, 3, 4, 5, 6, 7]


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def plain_boxes():
    vertical = px.box(y=VALUES)  # xpd: box-plot-ink
    horizontal = px.box(x=VALUES)  # xpd: box-plot-ink
    trace = go.Figure(go.Box(y=VALUES))  # xpd: box-plot-ink
    sideways = go.Figure(go.Box(x=VALUES))  # xpd: box-plot-ink
    express_alias = express.box(y=VALUES)  # xpd: box-plot-ink
    objects_alias = graphs.Figure(graphs.Box(y=VALUES))  # xpd: box-plot-ink
    return vertical, horizontal, trace, sideways, express_alias, objects_alias


@case
def deliberate_boxes():
    points = px.box(y=VALUES, points="all")
    hidden_points = go.Figure(go.Box(y=VALUES, boxpoints=False))
    notches = px.box(y=VALUES, notched=True)
    mean = go.Figure(go.Box(y=VALUES, boxmean=True))
    styled = go.Figure(go.Box(y=VALUES, whiskerwidth=0, width=0.04))
    summary = go.Figure(go.Box(q1=[2.5], median=[4], q3=[5.5], lowerfence=[1], upperfence=[7]))
    grouped = px.box(x=["a"] * 7, y=VALUES)
    colored = px.box(y=VALUES, color=["a"] * 7)
    options = {"boxpoints": "all"}
    unknown = go.Figure(go.Box(y=VALUES, **options))
    violin = go.Figure(go.Violin(y=VALUES))
    return points, hidden_points, notches, mean, styled, summary, grouped, colored, unknown, violin


@case
def quartile_drawing():
    # The endpoints are min/max here, so the drawing is for that statistic only.
    low, q1, median, q3, high = 1, 2.5, 4, 5.5, 7
    return go.Figure(go.Scatter(  # xpd: multi-line-call-argument
        x=[0, 0, None, 0.03, 0.03, None, 0.03, 0.03, None, 0, 0, None],
        y=[low, q1, None, q1, median - 0.05, None, median + 0.05, q3, None, q3, high, None],
        mode="lines",
    ))
