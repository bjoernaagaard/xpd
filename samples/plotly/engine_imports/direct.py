"""A script that imports the module that chooses the look.

No line of this file is reported. Each case returns the figures the code builds.
"""

import plotly.express as px

from house_look import PALETTE

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def look_by_import():
    days = {"day": ["Thu", "Fri", "Sat", "Sun"], "tip": [3.1, 2.7, 4.2, 3.9]}
    return px.scatter(days, x="day", y="tip", color_discrete_sequence=PALETTE)
