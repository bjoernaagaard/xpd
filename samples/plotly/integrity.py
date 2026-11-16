"""Figures whose drawn sizes do not follow the numbers, next to the forms that do.

A line that xpd reports ends with a comment that names the rule. Each case returns the
figures the code builds.
"""

import numpy as np
import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio
from plotly.subplots import make_subplots

pio.templates.default = "simple_white"

CASES = {}
MARKER_SIZE = 12


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def cities():
    return pd.DataFrame({
        "city": ["Oslo", "Bergen", "Tromso"],
        "pop": [700, 290, 78],
        "area": [454, 465, 2521],
        "rain": [763, 2250, 1031],
        "lon": [10.7, 5.3, 18.9],
        "lat": [59.9, 60.4, 69.6],
    })


@case
def marker_size():
    df = cities()
    sizes = [10, 20, 40]
    by_diameter = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=df["pop"]))  # xpd: marker-size-diameter
    scaled = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=df["pop"] / 10, color="gray"))  # xpd: marker-size-diameter
    in_dict = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker={"size": df["pop"].abs() / 5})  # xpd: marker-size-diameter
    underscore = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker_size=df["pop"].to_numpy())  # xpd: marker-size-diameter
    on_a_map = go.Scattergeo(lon=df["lon"], lat=df["lat"], marker=dict(size=df["pop"] / 10))  # xpd: marker-size-diameter
    two_sizes = go.Scatter(x=[1, 2], y=[1, 1], mode="markers", marker=dict(size=[12, 15]))
    glow = dict(size=30, opacity=0.2)
    other_source = go.Scatter(x=[1], y=[1], mode="markers", marker=dict(size=glow["size"] + 11, opacity=glow["opacity"]))
    by_condition = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=np.where(df["pop"] > 500, 14, 8)))
    one_size = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=12))
    named_size = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=MARKER_SIZE))
    a_name = go.Scatter(x=[1, 2, 3], y=[1, 1, 1], mode="markers", marker=dict(size=sizes))
    colored = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(color=df["pop"]))
    by_px = px.scatter(df, x="area", y="rain", size="pop")
    # A square root makes the area follow the value, and a column named size was made for the markers.
    rooted = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=np.sqrt(df["pop"])))
    powered = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker_size=df["pop"] ** 0.5)
    sized = df.assign(size=np.sqrt(df["pop"]))
    made_for_markers = go.Scatter(x=sized["area"], y=sized["rain"], mode="markers", marker=dict(size=sized["size"]))
    # The diameter written out is the same distortion, also on a px figure that had the area.
    written = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=df["pop"], sizemode="diameter"))  # xpd: marker-size-diameter
    turned_back = px.scatter(df, x="area", y="rain", size="pop")
    turned_back.update_traces(marker_sizemode="diameter")  # xpd: marker-size-diameter
    traces = (by_diameter, scaled, in_dict, underscore, on_a_map, two_sizes, other_source, by_condition, one_size, named_size, a_name, colored)
    traces += (rooted, powered, made_for_markers, written)
    return [go.Figure(t) for t in traces] + [by_px, turned_back]


@case
def bar_axis():
    df = pd.DataFrame({"team": ["a", "b"], "score": [95, 100]})
    low, high = 90, 101
    cut = px.bar(df, x="team", y="score", range_y=[90, 101])  # xpd: bar-axis-without-zero
    cut_flat = px.bar(df, x="score", y="team", orientation="h", range_x=[90, 101])  # xpd: bar-axis-without-zero
    counted = px.histogram(df, x="team", range_y=[1, 2])  # xpd: bar-axis-without-zero
    later = px.bar(df, x="team", y="score")
    later.update_yaxes(range=[90, 101])  # xpd: bar-axis-without-zero
    in_layout = px.bar(df, x="team", y="score").update_layout(yaxis_range=[90, 101])  # xpd: bar-axis-without-zero
    nested = go.Figure(data=[go.Bar(x=df["team"], y=df["score"])])
    nested.update_layout(yaxis=dict(range=[90.0, 101]))  # xpd: bar-axis-without-zero
    from_zero = px.bar(df, x="team", y="score", range_y=[0, 101])
    below_zero = px.bar(df, x="team", y="score", range_y=[-10, 101])
    from_names = px.bar(df, x="team", y="score", range_y=[low, high])
    nearly_zero = px.bar(df, x="team", y="score", range_y=[1, 101])
    points = px.scatter(df, x="team", y="score", range_y=[90, 101])
    line = px.line(df, x="team", y="score")
    line.update_yaxes(range=[90, 101])
    reused = px.bar(df, x="team", y="score")
    reused = px.line(df, x="team", y="score")
    reused.update_yaxes(range=[90, 101])
    in_constructor = go.Figure(data=[go.Bar(x=df["team"], y=df["score"])], layout=go.Layout(yaxis=dict(range=[90, 101])))  # xpd: bar-axis-without-zero
    # With base each bar starts at its base, so a range above zero cuts nothing.
    temps = pd.DataFrame({"month": ["jan", "feb"], "low": [91, 93], "spread": [4, 7]})
    floating = px.bar(temps, x="month", y="spread", base="low", range_y=[90, 101])
    figures = (cut, cut_flat, counted, later, in_layout, nested, from_zero, below_zero, from_names, nearly_zero, points, line, reused)
    return figures + (in_constructor, floating)


@case
def polar_bars():
    wind = pd.DataFrame({"direction": ["N", "E", "S", "W"], "hours": [1, 2, 3, 4]})
    rose = px.bar_polar(wind, r="hours", theta="direction")  # xpd: polar-bar-area
    rose_go = go.Figure(go.Barpolar(r=wind["hours"], theta=wind["direction"]))  # xpd: polar-bar-area
    bars = px.bar(wind, x="direction", y="hours")
    polar_points = px.scatter_polar(wind, r="hours", theta="direction")
    return rose, rose_go, bars, polar_points


@case
def third_dimension():
    df = cities()
    n = len(df)
    flat = px.scatter_3d(df, x="area", y="rain", z=[0] * n)  # xpd: three-d-for-two-d
    flat_go = go.Figure(go.Scatter3d(x=df["area"], y=df["rain"], z=np.zeros(n), mode="markers"))  # xpd: three-d-for-two-d
    three = px.scatter_3d(df, x="area", y="rain", z="pop")
    positional = px.scatter_3d(df, "area", "rain", "pop")
    two = px.scatter(df, x="area", y="rain")
    return flat, flat_go, three, positional, two


def empty_scene():
    # plotly draws nothing for these, so they are not a case.
    df = cities()
    missing = px.scatter_3d(df, x="area", y="rain")  # xpd: three-d-for-two-d
    missing_line = px.line_3d(df, x="area", y="rain")  # xpd: three-d-for-two-d
    missing_go = go.Scatter3d(x=df["area"], y=df["rain"], mode="markers")  # xpd: three-d-for-two-d
    return missing, missing_line, missing_go


@case
def two_scales():
    months = [1, 2, 3]
    dual = make_subplots(specs=[[{"secondary_y": True}]])  # xpd: dual-y-axis
    dual.add_trace(go.Scatter(x=months, y=[10, 20, 30], name="sales"), secondary_y=False)
    dual.add_trace(go.Scatter(x=months, y=[500, 400, 450], name="visits"), secondary_y=True)
    by_hand = go.Figure([go.Scatter(x=months, y=[10, 20, 30]), go.Scatter(x=months, y=[500, 400, 450], yaxis="y2")])
    by_hand.update_layout(yaxis2=dict(overlaying="y", side="right"))  # xpd: dual-y-axis
    tied = go.Figure([go.Scatter(x=months, y=[10, 20, 30]), go.Scatter(x=months, y=[12, 18, 33], yaxis="y2")])
    tied.update_layout(yaxis2=dict(overlaying="y", side="right", matches="y"))
    panels = make_subplots(rows=2, cols=1, shared_xaxes=True, specs=[[{"secondary_y": False}], [{}]])
    panels.add_trace(go.Scatter(x=months, y=[10, 20, 30], name="sales"), row=1, col=1)
    panels.add_trace(go.Scatter(x=months, y=[500, 400, 450], name="visits"), row=2, col=1)
    return dual, by_hand, tied, panels


@case
def facet_scales():
    tips = px.data.tips()
    free = px.scatter(tips, x="total_bill", y="tip", facet_col="sex")
    free.update_yaxes(matches=None)  # xpd: free-facet-scales
    free_chain = px.histogram(tips, x="tip", facet_row="smoker").update_xaxes(matches=None)  # xpd: free-facet-scales
    shared = px.scatter(tips, x="total_bill", y="tip", facet_col="sex")
    shared.update_yaxes(title_text="tip")
    one_panel = px.scatter(tips, x="total_bill", y="tip")
    one_panel.update_yaxes(matches=None)
    return free, free_chain, shared, one_panel
