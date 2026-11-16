"""Pie constructors keep their warning when the receiver has no figure proof."""

import plotly.graph_objects as go
import plotly.io as pio
from plotly.subplots import make_subplots

pio.templates.default = "simple_white"
VALUES = [1, 2]


def unknown_pies(fig):
    fig.add_trace(go.Pie(values=VALUES))  # xpd: pie-chart
    fig.add_trace(go.Pie(values=VALUES))  # xpd: pie-chart


def unknown_list(fig):
    fig.add_traces([
        go.Pie(values=VALUES),  # xpd: pie-chart
        go.Pie(values=VALUES),  # xpd: pie-chart
    ])


def unknown_loop(fig):
    for col in range(1, 3):
        fig.add_trace(go.Pie(values=VALUES), row=1, col=col)  # xpd: pie-chart


def called_pies(fig):
    fig.add_trace(go.Pie(values=VALUES))  # xpd: pie-chart
    fig.add_trace(go.Pie(values=VALUES))  # xpd: pie-chart


def known_caller():
    fig = go.Figure()
    called_pies(fig)
    return fig


def annotated_pies(fig: go.Figure):
    fig.add_trace(go.Pie(values=VALUES))  # xpd: pie-chart
    fig.add_trace(go.Pie(values=VALUES))  # xpd: pie-chart
    return fig


def listed_pies():
    fig = go.Figure(data=[
        go.Pie(values=VALUES),  # xpd: pie-chart
        go.Pie(values=VALUES),  # xpd: pie-chart
    ])
    return fig


def added_list():
    fig = go.Figure()
    fig.add_traces([
        go.Pie(values=VALUES),  # xpd: pie-chart
        go.Pie(values=VALUES),  # xpd: pie-chart
    ])
    return fig


def looped_pies():
    fig = make_subplots(rows=1, cols=2, specs=[[{"type": "domain"}, {"type": "domain"}]])
    for col in range(1, 3):
        fig.add_trace(go.Pie(values=VALUES), row=1, col=col)  # xpd: pie-chart
    return fig


CASES = {
    "caller": known_caller,
    "annotated": lambda: annotated_pies(go.Figure()),
    "listed": listed_pies,
    "added": added_list,
    "looped": looped_pies,
}
