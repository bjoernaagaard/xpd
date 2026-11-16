"""A script of go.Figure calls that imports the module that sets px.defaults.

px.defaults does not reach a go.Figure, so the figure below is drawn with the look plotly starts
with, and xpd reports the import of plotly.graph_objects. The comment on that line names the
rule. Each case returns the figures the code builds.
"""

import plotly.graph_objects as go  # xpd: default-template

import express_look  # for what it sets

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def look_plotly_starts_with():
    return go.Figure(go.Scatter(x=[1, 2, 3], y=[3.1, 2.7, 4.2]))
