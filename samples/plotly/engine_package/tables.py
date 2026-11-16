"""A module of the same package that builds its figures with go.Figure.

It does not import plotly.express, and without the __init__.py beside it xpd reports the import
of plotly.graph_objects, as it does in ../default_look_go.py. No line of this file is reported.
Each case returns the figures the code builds.
"""

from plotly import graph_objects as go

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def look_of_the_package():
    return go.Figure(go.Scatter(x=[1, 2, 3], y=[3.1, 2.7, 4.2]))
