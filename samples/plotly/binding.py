"""Names that look like plotly and are not, next to plotly under other names.

A rule reports a call only when the file binds the name in front of it to a plotly module,
and a method only on a variable that holds a plotly figure at that line. A line that xpd
reports ends with a comment that names the rule. Each case returns what the code builds.
"""

import plotly.express as express
import plotly.io
from plotly import graph_objects as graph
from plotly.subplots import make_subplots

plotly.io.templates.default = "simple_white"

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


class Drawing:
    """A stand-in for a figure of another library, with the method names of a plotly figure."""

    def __init__(self, **settings):
        self.settings = dict(settings)

    def update_xaxes(self, **settings):
        self.settings.update(settings)
        return self

    update_yaxes = update_layout = update_traces = update_xaxes


class px:
    """A local helper whose short name is the one plotly.express usually gets."""

    pie = bar = scatter = bar_polar = staticmethod(lambda *args, **settings: Drawing(**settings))


class go:
    Pie = Barpolar = Indicator = Figure = staticmethod(lambda *args, **settings: Drawing(**settings))


DAYS = {"day": ["Thu", "Fri", "Sat"], "tip": [3.1, 2.7, 4.2]}


@case
def other_names():
    real_pie = express.pie(DAYS, names="day", values="tip")  # xpd: pie-chart
    dotted_pie = plotly.express.pie(DAYS, names="day", values="tip")  # xpd: pie-chart
    real_rose = graph.Figure(graph.Barpolar(r=[1, 2], theta=[0, 90]))  # xpd: polar-bar-area
    real_bar = express.bar(DAYS, x="day", y="tip", range_y=[2, 5])  # xpd: bar-axis-without-zero
    local_pie = px.pie(DAYS, names="day", values="tip")
    local_rose = go.Figure(go.Barpolar(r=[1, 2], theta=[0, 90]))
    local_bar = px.bar(DAYS, x="day", y="tip", range_y=[2, 5])
    local_dial = go.Indicator(mode="gauge+number", value=3)
    return real_pie, dotted_pie, real_rose, real_bar, local_pie.settings, local_rose.settings, local_bar.settings, local_dial.settings


@case
def figure_variables():
    fig = express.scatter(DAYS, x="day", y="tip")
    fig.update_xaxes(tickangle=-90)  # xpd: rotated-tick-labels
    fig = Drawing()
    fig.update_xaxes(tickangle=-90)
    fig = make_subplots(rows=1, cols=2)
    fig.update_xaxes(showgrid=True)  # xpd: explicit-grid
    drawing = px.scatter(DAYS, x="day", y="tip")
    drawing.update_xaxes(tickangle=-90, showgrid=True, showline=True, mirror=True)
    drawing.update_layout(template="ggplot2", yaxis2=dict(overlaying="y"))
    drawing.update_traces(line_shape="spline")
    figures = [express.scatter(DAYS, x="day", y="tip"), Drawing()]
    for fig in figures:
        fig.update_xaxes(tickangle=-90)
    return [type(f).__name__ for f in figures], drawing.settings


def tilt(fig):
    # One call below passes a Drawing, so the parameter is left alone.
    return fig.update_xaxes(tickangle=-90)


@case
def parameters():
    return tilt(express.scatter(DAYS, x="day", y="tip")), tilt(Drawing()).settings
