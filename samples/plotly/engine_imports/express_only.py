"""A script of px figures that imports the module that sets px.defaults.

No line of this file is reported. Each case returns the figures the code builds.
"""

import plotly.express as px

import express_look  # for what it sets

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def look_of_px():
    days = {"day": ["Thu", "Fri", "Sat", "Sun"], "tip": [3.1, 2.7, 4.2, 3.9]}
    return px.scatter(days, x="day", y="tip")
