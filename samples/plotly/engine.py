"""What xpd proves about the variable a method is called on.

A rule about a method, such as fig.update_xaxes(...), reports only on a plotly figure. xpd
follows the variable back to the nearest earlier statement that binds it, however far back
that is, and follows a parameter to its annotation or out to every call of its function. A
line that xpd reports ends with a comment that names the rule. Each case returns what the
code builds.
"""

import functools
import json

import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio

pio.templates.default = "simple_white"

CASES = {}
DAYS = {"day": ["Thu", "Fri", "Sat"], "tip": [3.1, 2.7, 4.2]}
SCORES = {"team": ["a", "b"], "score": [95, 100]}


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

    update_yaxes = update_layout = update_xaxes


@case
def far_from_the_assignment():
    fig = px.scatter(DAYS, x="day", y="tip")
    bars = px.bar(SCORES, x="team", y="score")
    low = min(DAYS["tip"])
    high = max(DAYS["tip"])
    spread = high - low
    middle = (high + low) / 2
    share = [tip / high for tip in DAYS["tip"]]
    best = DAYS["day"][DAYS["tip"].index(high)]
    worst = DAYS["day"][DAYS["tip"].index(low)]
    title = f"{best} beats {worst} by {spread:.1f}"
    notes = [title, f"middle {middle:.1f}"]
    notes.append(f"shares {share}")
    width = 80 * len(DAYS["day"])
    height = width // 2
    size = dict(width=width, height=height)
    fig.update_layout(title_text=title, **size)
    fig.update_xaxes(tickangle=-90)  # xpd: rotated-tick-labels
    if spread > 1:
        for _ in notes:
            fig.update_yaxes(showgrid=True)  # xpd: explicit-grid
    bars.update_yaxes(range=[90, 101])  # xpd: bar-axis-without-zero
    return fig, bars


@case
def the_nearest_assignment_decides():
    fig = px.scatter(DAYS, x="day", y="tip")
    fig = Drawing()
    fig.update_xaxes(tickangle=-90)
    drawing = fig
    fig = px.scatter(DAYS, x="day", y="tip")
    fig.update_xaxes(tickangle=-90)  # xpd: rotated-tick-labels
    # A variable assigned from another one holds what that one held at the assignment.
    same = fig
    same.update_xaxes(tickangle=45)  # xpd: rotated-tick-labels
    drawing.update_xaxes(tickangle=45)
    return fig, drawing.settings


@case
def a_binding_that_is_not_an_assignment():
    fig = px.scatter(DAYS, x="day", y="tip")
    if len(DAYS["day"]) > 5:
        fig = Drawing()
    # The branch above may have run, so the assignment before it proves nothing.
    fig.update_xaxes(tickangle=-90)
    chart = px.scatter(DAYS, x="day", y="tip")
    for chart in [chart, Drawing()]:
        chart.update_xaxes(tickangle=-90)
    plot = px.scatter(DAYS, x="day", y="tip")
    import json as plot

    if hasattr(plot, "update_xaxes"):
        plot.update_xaxes(tickangle=-90)
    blank = Drawing()
    turned = (lambda blank: blank.update_xaxes(tickangle=-90))(blank)

    def later():
        # The function runs after the code around it, so `fig` may hold something else by then.
        return fig.update_xaxes(tickangle=-90)

    return later(), type(chart).__name__, plot is json, turned.settings


@case
def an_annotation_on_the_assignment():
    loaded: go.Figure = pio.from_json(px.scatter(DAYS, x="day", y="tip").to_json())
    loaded.update_xaxes(tickangle=-90)  # xpd: rotated-tick-labels
    # The class of a figure does not say that it holds bars.
    loaded.update_yaxes(range=[90, 101])
    return loaded


def tilt(fig: go.Figure):
    return fig.update_xaxes(tickangle=-90)  # xpd: rotated-tick-labels


def grid(fig, wide=False, *, color="#eee"):
    fig.update_xaxes(showgrid=True)  # xpd: explicit-grid
    return fig.update_yaxes(tickangle=30, title_standoff=20 if wide else 10, gridcolor=color)  # xpd: rotated-tick-labels


def cut(chart):
    return chart.update_yaxes(range=[90, 101])  # xpd: bar-axis-without-zero


def frame(*, fig):
    return fig.update_xaxes(showline=True, mirror=True)  # xpd: boxed-axis-frame


def outer(first):
    return inner(first)


def inner(second):
    return second.update_xaxes(tickangle=-90)  # xpd: rotated-tick-labels


@case
def a_figure_passed_to_a_function():
    fig = px.scatter(DAYS, x="day", y="tip")
    grid(fig)
    grid(wide=True, fig=px.line(DAYS, x="day", y="tip"))
    return (
        tilt(px.scatter(DAYS, x="day", y="tip")),
        fig,
        cut(px.bar(SCORES, x="team", y="score")),
        frame(fig=go.Figure(go.Scatter(x=[1, 2], y=[3, 4]))),
        outer(px.scatter(DAYS, x="day", y="tip")),
    )


def mixed(fig):
    # One call below passes a Drawing, so the parameter is left alone.
    return fig.update_xaxes(tickangle=-90)


def never_called(fig):
    return fig.update_xaxes(tickangle=-90)


def handed_on(fig):
    # map() calls the function, and the file does not show with what.
    return fig.update_xaxes(tickangle=-90)


def rebound(fig):
    fig = Drawing()
    return fig.update_xaxes(tickangle=-90)


def spread(fig):
    return fig.update_xaxes(tickangle=-90)


def defaulted(fig=None):
    return (fig or Drawing()).update_xaxes(tickangle=-90), fig and fig.update_xaxes(tickangle=-90)


def not_bars(chart):
    # Every call passes a plotly figure, and one of them is not a bar chart.
    return chart.update_yaxes(range=[90, 101])


def counted(fn):
    @functools.wraps(fn)
    def run(*args):
        run.calls += 1
        return fn(*args)

    run.calls = 0
    return run


@counted
def wrapped(fig):
    # A decorator puts another function under the name, which can pass on anything.
    return fig.update_xaxes(tickangle=-90)


class Report:
    def style(self, fig):
        # A method is called through an object, and the file does not show which class it has.
        return fig.update_xaxes(tickangle=-90)


@case
def a_parameter_without_proof():
    fig = px.scatter(DAYS, x="day", y="tip")
    options = {"fig": px.scatter(DAYS, x="day", y="tip")}
    handed = list(map(handed_on, [px.scatter(DAYS, x="day", y="tip")]))
    return (
        mixed(fig),
        mixed(Drawing()).settings,
        handed,
        rebound(fig).settings,
        spread(**options),
        defaulted()[0].settings,
        not_bars(px.bar(SCORES, x="team", y="score")),
        not_bars(px.scatter(DAYS, x="day", y="tip")),
        wrapped(fig),
        Report().style(fig),
    )
