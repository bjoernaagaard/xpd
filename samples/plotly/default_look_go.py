"""A file that builds its figures with go.Figure and draws with the look plotly starts with.

The file does not import plotly.express, so xpd reports it once, on the import of
plotly.graph_objects. The comment on that line names the rule. Each case returns the figures
the code builds.
"""

from plotly import graph_objects as go  # xpd: default-template

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def default_look():
    return go.Figure(go.Scatter(x=[1, 2, 3], y=[3.1, 2.7, 4.2]))
