"""A file that draws with the look plotly starts with.

Nothing here chooses the look of the figures, so xpd reports the file once, on the import of
plotly.express. The comment on that line names the rule. Each case returns the figures the
code builds.
"""

import pandas as pd
import plotly.express as px  # xpd: default-template
import plotly.graph_objects as go

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def default_look():
    df = pd.DataFrame({"day": ["Thu", "Fri", "Sat", "Sun"], "tip": [3.1, 2.7, 4.2, 3.9]})
    return px.scatter(df, x="day", y="tip"), go.Figure(go.Scatter(x=df["day"], y=df["tip"]))
