"""A script in the same folder that imports no module of the project.

A file beside it chooses a look, and nothing brings that file into this one, so xpd reports the
import of plotly.express. The comment on that line names the rule. Each case returns the
figures the code builds.
"""

import plotly.express as px  # xpd: default-template

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def look_plotly_starts_with():
    days = {"day": ["Thu", "Fri", "Sat", "Sun"], "tip": [3.1, 2.7, 4.2, 3.9]}
    return px.scatter(days, x="day", y="tip")
