"""A script that reaches the look through a helper module: shared.py imports house_look.py.

No line of this file is reported. Each case returns the figures the code builds.
"""

import plotly.graph_objects as go

import shared

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def look_two_imports_away():
    return go.Figure(go.Scatter(x=shared.DAYS["day"], y=shared.DAYS["tip"], marker_color=shared.COLORS[0]))
