"""Forms that four rules leave alone for the whole file.

`marker-size-diameter` is silent in a file that names sizemode with another value than
"diameter", `bar-axis-without-zero` reads no range set on a figure in a file that names a log
axis, `legend-instead-of-labels` is silent in a file that names showlegend, an annotation or a
text mode, and `three-d-for-two-d` reports no missing z in a file that sets z on an object. The
size mode, the axis type, the labels and z can be set after the call that the rule reads, so
one mention ends the proof. No line of this file is reported. Each case returns the figures the code
builds.
"""

import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio

pio.templates.default = "simple_white"

CASES = {}
YEARS = [2021, 2022, 2023]


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def bubbles_by_area():
    df = pd.DataFrame({"area": [454, 465, 2521], "rain": [763, 2250, 1031], "pop": [700, 290, 78]})
    stated = go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=df["pop"], sizemode="area", sizeref=2 * df["pop"].max() / 40**2))
    later = go.Figure(go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=df["pop"])))
    later.update_traces(marker_sizemode="area", marker_sizeref=2 * df["pop"].max() / 40**2)
    return go.Figure(stated), later


@case
def bars_on_a_log_axis():
    df = pd.DataFrame({"team": ["a", "b"], "score": [95, 100]})
    fig = px.bar(df, x="team", y="score")
    fig.update_yaxes(type="log")
    fig.update_yaxes(range=[1, 3])
    return fig


@case
def z_set_afterwards():
    trace = go.Scatter3d(x=[1, 2, 3], y=[1, 2, 3], mode="markers")
    trace.z = [3, 1, 2]
    return go.Figure(trace)


@case
def labelled_lines():
    fig = go.Figure(data=[
        go.Scatter(x=YEARS, y=[1, 2, 4], name="north", mode="lines+text", text=["", "", "north"], textposition="middle right"),
        go.Scatter(x=YEARS, y=[2, 2, 3], name="south", mode="lines+text", text=["", "", "south"], textposition="middle right"),
    ])
    fig.update_layout(showlegend=False)
    plain = go.Figure(data=[go.Scatter(x=YEARS, y=[1, 2, 4], name="north"), go.Scatter(x=YEARS, y=[2, 2, 3], name="south")])
    plain.update_layout(showlegend=False)
    return fig, plain
