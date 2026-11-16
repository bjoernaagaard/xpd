"""Figures built from go traces, added one by one, next to the forms that read the same.

A line that xpd reports ends with a comment that names the rule. Each case returns the
figures the code builds.
"""

import types

import numpy as np
import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio
from plotly.subplots import make_subplots

pio.templates.default = "simple_white"

CASES = {}
YEARS = [2021, 2022, 2023]
LABELS = ["rent", "food"]
DOMAINS = [[{"type": "domain"}, {"type": "domain"}]]


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def scores():
    return pd.DataFrame({"team": ["a", "b"], "score": [95, 100], "other": [3, 4]})


@case
def bars_added_later():
    df = scores()
    added = go.Figure()
    added.add_trace(go.Bar(x=df["team"], y=df["score"]))
    added.update_yaxes(range=[90, 101])  # xpd: bar-axis-without-zero
    by_add_bar = go.Figure()
    by_add_bar.add_bar(x=df["team"], y=df["score"])
    by_add_bar.update_layout(yaxis_range=[90, 101])  # xpd: bar-axis-without-zero
    looped = go.Figure()
    for team, part in df.groupby("team"):
        looped.add_trace(go.Bar(x=part["team"], y=part["score"], name=team))
    looped.update_layout(yaxis=dict(range=[90, 101]))  # xpd: bar-axis-without-zero
    lying = go.Figure()
    lying.add_trace(go.Bar(x=df["score"], y=df["team"], orientation="h"))
    lying.update_yaxes(range=[90, 101])
    points = go.Figure()
    points.add_trace(go.Scatter(x=df["team"], y=df["score"]))
    points.update_yaxes(range=[90, 101])
    renamed = go.Figure()
    renamed.add_trace(go.Bar(x=df["team"], y=df["score"]))
    renamed = go.Figure()
    renamed.add_trace(go.Scatter(x=df["team"], y=df["score"]))
    renamed.update_yaxes(range=[90, 101])
    from_zero = go.Figure()
    from_zero.add_trace(go.Bar(x=df["team"], y=df["score"]))
    from_zero.update_yaxes(range=[0, 101])
    return added, by_add_bar, looped, lying, points, renamed, from_zero


@case
def frames():
    points = [(1, 1), (2, 3), (3, 2)]
    steps = [go.Frame(data=[go.Scatter(x=[x], y=[y], mode="markers")], name=str(x)) for x, y in points]
    first = [go.Scatter(x=[1], y=[1], mode="markers")]
    built = go.Figure(data=first, frames=steps)  # xpd: animation-frame
    later = go.Figure(data=first)
    later.frames = steps  # xpd: animation-frame
    updated = go.Figure(data=first)
    updated.update(frames=steps)  # xpd: animation-frame
    copied = go.Figure(data=first, frames=built.frames)
    none_given = go.Figure(data=first, frames=None)
    empty = go.Figure(data=first, frames=[])
    other = types.SimpleNamespace()
    other.frames = steps
    panels = make_subplots(rows=1, cols=3)
    for i, (x, y) in enumerate(points):
        panels.add_trace(go.Scatter(x=[x], y=[y], mode="markers"), row=1, col=i + 1)
    return built, later, updated, copied, none_given, empty, other.frames, panels


@case
def several_pies():
    listed = go.Figure([
        go.Pie(labels=LABELS, values=[1, 2], domain=dict(x=[0, 0.45])),  # xpd: pie-chart
        go.Pie(labels=LABELS, values=[2, 1], domain=dict(x=[0.55, 1])),  # xpd: pie-chart
    ])
    cells = make_subplots(rows=1, cols=2, specs=DOMAINS)
    cells.add_trace(go.Pie(labels=LABELS, values=[1, 2]), row=1, col=1)  # xpd: pie-chart
    cells.add_trace(go.Pie(labels=LABELS, values=[2, 1]), row=1, col=2)  # xpd: pie-chart
    looped = make_subplots(rows=1, cols=2, specs=DOMAINS)
    for i, values in enumerate(([1, 2], [2, 1])):
        looped.add_trace(go.Pie(labels=LABELS, values=values), row=1, col=i + 1)  # xpd: pie-chart
    one = go.Figure(go.Pie(labels=LABELS, values=[1, 2]))  # xpd: pie-chart
    pie_and_bar = make_subplots(rows=1, cols=2, specs=[[{"type": "domain"}, {"type": "xy"}]])
    pie_and_bar.add_trace(go.Pie(labels=LABELS, values=[1, 2]), row=1, col=1)  # xpd: pie-chart
    pie_and_bar.add_trace(go.Bar(x=LABELS, y=[1, 2]), row=1, col=2)
    reused = go.Figure()
    reused.add_trace(go.Pie(labels=LABELS, values=[1, 2]))  # xpd: pie-chart
    reused = go.Figure()
    reused.add_trace(go.Pie(labels=LABELS, values=[2, 1]))  # xpd: pie-chart
    return listed, cells, looped, one, pie_and_bar, reused


def tips_by_size():
    return px.data.tips().groupby(["size", "sex", "smoker"], as_index=False)["tip"].mean()


@case
def legends_added():
    added = go.Figure()
    added.add_trace(go.Scatter(x=YEARS, y=[1, 2, 4], name="north"))  # xpd: legend-instead-of-labels
    added.add_trace(go.Scatter(x=YEARS, y=[2, 2, 3], name="south"))
    four = go.Figure(layout=dict(title="four regions"))
    four.add_trace(go.Scatter(x=YEARS, y=[1, 2, 4], name="a"))  # xpd: legend-instead-of-labels
    four.add_trace(go.Scatter(x=YEARS, y=[2, 2, 3], name="b"))
    four.add_trace(go.Scatter(x=YEARS, y=[3, 2, 3], name="c"))
    four.add_trace(go.Scatter(x=YEARS, y=[4, 2, 3], name="d"))
    five = go.Figure()
    five.add_trace(go.Scatter(x=YEARS, y=[1, 2, 4], name="a"))
    five.add_trace(go.Scatter(x=YEARS, y=[2, 2, 3], name="b"))
    five.add_trace(go.Scatter(x=YEARS, y=[3, 2, 3], name="c"))
    five.add_trace(go.Scatter(x=YEARS, y=[4, 2, 3], name="d"))
    five.add_trace(go.Scatter(x=YEARS, y=[5, 2, 3], name="e"))
    with_bars = go.Figure()
    with_bars.add_trace(go.Scatter(x=YEARS, y=[1, 2, 4], name="north"))
    with_bars.add_trace(go.Scatter(x=YEARS, y=[2, 2, 3], name="south"))
    with_bars.add_trace(go.Bar(x=YEARS, y=[3, 3, 3], name="total"))
    looped = go.Figure()
    for region, values in (("north", [1, 2, 4]), ("south", [2, 2, 3])):
        looped.add_trace(go.Scatter(x=YEARS, y=values, name=region))
    one_each = go.Figure()
    one_each.add_trace(go.Scatter(x=YEARS, y=[1, 2, 4], name="north"))
    one_each = go.Figure()
    one_each.add_trace(go.Scatter(x=YEARS, y=[2, 2, 3], name="south"))
    points = go.Figure()
    points.add_trace(go.Scatter(x=YEARS, y=[1, 2, 4], name="north", mode="markers"))
    points.add_trace(go.Scatter(x=YEARS, y=[2, 2, 3], name="south", mode="markers"))
    started = go.Figure(go.Scatter(x=YEARS, y=[3, 3, 3], name="total"))
    started.add_trace(go.Scatter(x=YEARS, y=[1, 2, 4], name="north"))
    started.add_trace(go.Scatter(x=YEARS, y=[2, 2, 3], name="south"))
    return added, four, five, with_bars, looped, one_each, points, started


@case
def px_legends():
    df = tips_by_size()
    by_sex = px.line(df[df["smoker"] == "No"], x="size", y="tip", color="sex")  # xpd: legend-instead-of-labels
    wide = pd.DataFrame({"t": [1, 2, 3], "a": [1, 2, 4], "b": [2, 2, 3]})
    columns = px.line(wide, x="t", y=["a", "b"])  # xpd: legend-instead-of-labels
    one_column = px.line(wide, x="t", y=["a"])
    grouped = px.line(df, x="size", y="tip", color="sex", line_group="smoker")
    key = "sex"
    by_name = px.line(df[df["smoker"] == "No"], x="size", y="tip", color=key)
    single = px.line(wide, x="t", y="a")
    return by_sex, columns, one_column, grouped, by_name, single


@case
def bar_colors():
    df = scores()
    values = [1, 2, 4]
    twice = go.Figure(go.Bar(x=df["team"], y=df["score"], marker_color=df["score"]))  # xpd: redundant-color
    in_marker = go.Figure(go.Bar(x=["a", "b", "c"], y=values, marker=dict(color=values)))  # xpd: redundant-color
    in_dict = go.Figure(go.Bar(x=df["score"], y=df["team"], orientation="h", marker={"color": df["score"]}))  # xpd: redundant-color
    one_color = go.Figure(go.Bar(x=df["team"], y=df["score"], marker_color="gray"))
    another = go.Figure(go.Bar(x=df["team"], y=df["score"], marker_color=df["other"]))
    by_hand = go.Figure(go.Bar(x=df["team"], y=df["score"], marker=dict(color=["#555", "#aaa"])))
    return twice, in_marker, in_dict, one_color, another, by_hand


@case
def marker_objects():
    df = pd.DataFrame({"area": [454, 465, 2521], "rain": [763, 2250, 1031], "pop_total": [700, 290, 78], "marker_size": [26, 17, 9]})
    by_attribute = go.Scatter(x=df.area, y=df.rain, mode="markers", marker=dict(size=df.pop_total / 20))  # xpd: marker-size-diameter
    marker_object = go.Scatter(x=df.area, y=df.rain, mode="markers", marker=go.scatter.Marker(size=df.pop_total / 20))  # xpd: marker-size-diameter
    subscript_object = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=go.scatter.Marker(size=df["pop_total"] / 20))  # xpd: marker-size-diameter
    one_size = go.Scatter(x=df.area, y=df.rain, mode="markers", marker=go.scatter.Marker(size=12))
    made_for_markers = go.Scatter(x=df.area, y=df.rain, mode="markers", marker=dict(size=df.marker_size))
    a_property = go.Scatter(x=df.area, y=df.rain, mode="markers", marker=dict(size=df.size / 2))
    rooted = go.Scatter(x=df.area, y=df.rain, mode="markers", marker=go.scatter.Marker(size=np.sqrt(df.pop_total)))
    steps = np.arange(3)
    filled = go.Scatter(x=steps, y=steps, mode="markers", marker=dict(size=np.full(3, 10)))
    traces = (by_attribute, marker_object, subscript_object, one_size, made_for_markers, a_property, rooted, filled)
    return [go.Figure(t) for t in traces]


@case
def shared_cells():
    # A make_subplots that names shared_yaxes keeps one scale, so the loop is left alone.
    sales = pd.DataFrame({"shop": ["north", "north", "south", "south"], "month": [1, 2, 1, 2], "sales": [5, 6, 10, 12]})
    shared = make_subplots(rows=1, cols=2, shared_yaxes="all")
    for i, (shop, part) in enumerate(sales.groupby("shop")):
        shared.add_trace(go.Bar(x=part["month"], y=part["sales"], name=shop), row=1, col=i + 1)
    return shared
