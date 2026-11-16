"""A file that draws with the plot accessor and sets no plotting backend.

pandas then draws with matplotlib. Every plot call of the file passes only x, y and title and
stands alone, so the setting changes no other line, and xpd reports the import of pandas, once.
"""

import pandas as pd  # xpd: plot-without-plotly-backend

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def scores():
    df = pd.DataFrame({"team": ["a", "b", "c"], "score": [3, 5, 4], "cost": [1.0, 2.5, 2.0]})
    df.plot.bar(x="team", y="score", title="Score by team")
    df.plot.scatter("score", "cost")
    return len(df)


@case
def returned():
    df = pd.DataFrame({"team": ["a", "b", "c"], "score": [3, 5, 4]})
    return df.plot.line(x="team", y="score")
