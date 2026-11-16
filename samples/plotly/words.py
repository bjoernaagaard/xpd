"""Chart forms the book argues against, and words the reader has to work for.

A line that xpd reports ends with a comment that names the rule. Each case returns the
figures the code builds.
"""

import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio

pio.templates.default = "simple_white"

CASES = {}
ANGLE = -45


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def survey():
    return pd.DataFrame({"answer": ["strongly agree with the plan", "neither agree nor disagree", "strongly disagree"], "share": [41, 37, 22]})


@case
def tick_labels():
    df = survey()
    upright = px.bar(df, x="answer", y="share")
    upright.update_xaxes(tickangle=-90)  # xpd: rotated-tick-labels
    slanted = px.bar(df, x="answer", y="share").update_layout(xaxis_tickangle=45)  # xpd: rotated-tick-labels
    nested = go.Figure(go.Bar(x=df["answer"], y=df["share"]), layout=dict(xaxis=dict(tickangle=270)))  # xpd: rotated-tick-labels
    in_dict = px.bar(df, x="answer", y="share")
    in_dict.update_layout(xaxis={"tickangle": 30.0})  # xpd: rotated-tick-labels
    level = px.bar(df, x="share", y="answer", orientation="h")
    level.update_yaxes(tickangle=0)
    by_name = px.bar(df, x="answer", y="share")
    by_name.update_xaxes(tickangle=ANGLE)
    settings = dict(tickangle=-90)
    bar = go.Figure(go.Heatmap(z=[[1, 2], [3, 4]], colorbar=dict(tickangle=-90)))
    return upright, slanted, nested, in_dict, level, by_name, settings, bar


@case
def splines():
    readings = [0, 0, 10, 10, 0, 0]
    hours = list(range(6))
    curved = px.line(x=hours, y=readings, line_shape="spline")  # xpd: spline-line
    curved_go = go.Figure(go.Scatter(x=hours, y=readings, line_shape="spline"))  # xpd: spline-line
    nested = go.Figure(go.Scatter(x=hours, y=readings, line=dict(shape="spline", smoothing=1.3)))  # xpd: spline-line
    in_dict = go.Figure(go.Scatter(x=hours, y=readings, line={"shape": "spline"}))  # xpd: spline-line
    later = px.line(x=hours, y=readings)
    later.update_traces(line_shape="spline")  # xpd: spline-line
    straight = px.line(x=hours, y=readings, line_shape="linear")
    steps = go.Figure(go.Scatter(x=hours, y=readings, line=dict(shape="hv")))
    shape = "spline"
    by_name = px.line(x=hours, y=readings, line_shape=shape)
    return curved, curved_go, nested, in_dict, later, straight, steps, by_name


@case
def frames_over_time():
    gap = px.data.gapminder()
    gap = gap[gap["year"] >= 1992]
    movie = px.scatter(gap, x="gdpPercap", y="lifeExp", animation_frame="year", log_x=True)  # xpd: animation-frame
    multiples = px.scatter(gap, x="gdpPercap", y="lifeExp", facet_col="year", facet_col_wrap=2, log_x=True)
    frame = None
    still = px.scatter(gap, x="gdpPercap", y="lifeExp", animation_frame=frame, log_x=True)
    # px.scatter_3d has no facet_col, so the message names make_subplots for it.
    movie_3d = px.scatter_3d(gap, x="gdpPercap", y="lifeExp", z="pop", animation_frame="year")  # xpd: animation-frame
    return movie, multiples, still, movie_3d


@case
def pies():
    df = pd.DataFrame({"day": ["Thu", "Fri", "Sat", "Sun"], "shop": ["north", "south", "north", "south"], "tip": [3.1, 2.7, 4.2, 3.9]})
    pie = px.pie(df, names="day", values="tip")  # xpd: pie-chart
    donut = go.Figure(go.Pie(labels=df["day"], values=df["tip"], hole=0.5))  # xpd: pie-chart
    several = px.pie(df, names="day", values="tip", facet_col="shop")  # xpd: pie-chart
    one = px.pie(df, names="day", values="tip", facet_col=None)  # xpd: pie-chart
    bars = px.bar(df.sort_values("tip"), x="tip", y="day", orientation="h")
    table = go.Figure(go.Table(header=dict(values=["day", "tip"]), cells=dict(values=[df["day"], df["tip"]])))
    return pie, donut, several, one, bars, table
