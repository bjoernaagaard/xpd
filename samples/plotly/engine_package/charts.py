"""A module of a package whose __init__.py chooses the look.

Nothing here chooses a look, and without the __init__.py beside it xpd reports the import of
plotly.express, as it does in ../default_look.py. No line of this file is reported. Each case
returns the figures the code builds.
"""

import plotly.express as px

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def look_of_the_package():
    days = {"day": ["Thu", "Fri", "Sat", "Sun"], "tip": [3.1, 2.7, 4.2, 3.9]}
    return px.scatter(days, x="day", y="tip")
