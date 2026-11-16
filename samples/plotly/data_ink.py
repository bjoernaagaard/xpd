"""Figures that spend ink on something other than data, next to the forms that do not.

A line that xpd reports ends with a comment that names the rule. Each case returns the
figures the code builds.
"""

import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio

pio.templates.default = "simple_white"

CASES = {}
THEME = "ggplot2"


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def sales():
    return pd.DataFrame({"day": ["Thu", "Fri", "Sat", "Sun"], "shop": ["north", "south", "north", "south"], "tip": [3.1, 2.7, 4.2, 3.9]})


@case
def tinted_templates():
    df = sales()
    gray = px.scatter(df, x="day", y="tip", template="ggplot2")  # xpd: default-template
    later = px.scatter(df, x="day", y="tip")
    later.update_layout(template="seaborn")  # xpd: default-template
    in_layout = go.Figure(go.Scatter(x=df["day"], y=df["tip"]), layout=go.Layout(template="plotly"))  # xpd: default-template
    white = px.scatter(df, x="day", y="tip", template="simple_white")
    pale = px.scatter(df, x="day", y="tip", template="plotly_white")
    dark = px.scatter(df, x="day", y="tip", template="plotly_dark")
    named = px.scatter(df, x="day", y="tip", template=THEME)
    other = dict(template="ggplot2")
    return gray, later, in_layout, white, pale, dark, named, other["template"]


def session_default():
    # Not a case, because it changes the default of every figure made after it.
    pio.templates.default = "seaborn"  # xpd: default-template
    pio.templates.default = "simple_white"


@case
def grids():
    df = sales()
    fig = px.scatter(df, x="day", y="tip")
    fig.update_xaxes(showgrid=True)  # xpd: explicit-grid
    fig.update_layout(yaxis_showgrid=True)  # xpd: explicit-grid
    nested = go.Figure(go.Scatter(x=df["day"], y=df["tip"]), layout=go.Layout(xaxis=dict(showgrid=True), yaxis={"showgrid": True}))  # xpd: explicit-grid, explicit-grid
    heavy = px.scatter(df, x="day", y="tip")
    heavy.update_yaxes(gridcolor="black")  # xpd: explicit-grid
    heavy.update_layout(xaxis=dict(gridcolor="#000"))  # xpd: explicit-grid
    heavy.update_xaxes(gridcolor="gray", gridwidth=2)
    muted = px.scatter(df, x="day", y="tip")
    muted.update_yaxes(showgrid=True, gridcolor="#eee")
    muted.update_xaxes(showgrid=False)
    flag = True
    muted.update_xaxes(showgrid=flag, gridwidth=1)
    muted.update_yaxes(showgrid=False, gridcolor="black")
    globe = go.Figure(go.Scattergeo(lon=[10.7], lat=[59.9]))
    globe.update_geos(lataxis_showgrid=True)
    globe.update_layout(geo=dict(lonaxis=dict(showgrid=True)))
    settings = dict(showgrid=True)
    return fig, nested, heavy, muted, globe, settings


@case
def frames():
    df = sales()
    boxed = px.scatter(df, x="day", y="tip")
    boxed.update_xaxes(showline=True, mirror=True)  # xpd: boxed-axis-frame
    boxed.update_layout(yaxis=dict(showline=True, mirror="allticks"))  # xpd: boxed-axis-frame
    open_frame = px.scatter(df, x="day", y="tip")
    open_frame.update_xaxes(showline=True)
    open_frame.update_yaxes(showline=True, mirror=False)
    open_frame.update_yaxes(mirror=True)
    return boxed, open_frame


@case
def color_twice():
    df = sales()
    twice = px.bar(df, x="day", y="tip", color="day")  # xpd: redundant-color
    twice_y = px.scatter(df, x="day", y="tip", color="tip")  # xpd: redundant-color
    once = px.bar(df, x="day", y="tip")
    third = px.bar(df, x="day", y="tip", color="shop")
    column = "day"
    by_name = px.bar(df, x="day", y="tip", color=column)
    return twice, twice_y, once, third, by_name


@case
def hatching():
    df = sales()
    hatched = px.bar(df, x="day", y="tip", pattern_shape="shop")  # xpd: pattern-fill
    hatched_go = go.Figure(go.Bar(x=df["day"], y=df["tip"], marker_pattern_shape="/"))  # xpd: pattern-fill
    nested = go.Figure(go.Bar(x=df["day"], y=df["tip"], marker=dict(pattern=dict(shape=["/", "x", "", "."]))))  # xpd: pattern-fill
    area = go.Figure(go.Scatter(x=df["day"], y=df["tip"], fill="tozeroy", fillpattern=dict(shape="x")))  # xpd: pattern-fill
    tinted = px.bar(df, x="day", y="tip", color="shop", color_discrete_sequence=["#555", "#aaa"])
    no_shape = go.Figure(go.Bar(x=df["day"], y=df["tip"], marker_pattern_shape=""))
    line = go.Figure(go.Scatter(x=df["day"], y=df["tip"], line=dict(shape="hv")))
    return hatched, hatched_go, nested, area, tinted, no_shape, line


@case
def gauges():
    dial = go.Figure(go.Indicator(mode="gauge+number", value=270, gauge=dict(axis=dict(range=[0, 500]))))  # xpd: gauge-indicator
    number = go.Figure(go.Indicator(mode="number+delta", value=270, delta=dict(reference=250)))
    default_mode = go.Figure(go.Indicator(value=270, gauge=dict(axis=dict(range=[0, 500]))))
    bullet = go.Figure(go.Indicator(mode="number+gauge", value=270, gauge=dict(shape="bullet", axis=dict(range=[0, 500]))))
    return dial, number, default_mode, bullet
