"""A file whose plot calls need matplotlib, so xpd does not ask for another backend.

figsize belongs to matplotlib, the result of the second call is an Axes whose method the next
line calls, and plotly draws no pie. No line of this file is reported.
"""

import pandas as pd

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def scores():
    df = pd.DataFrame({"team": ["a", "b", "c"], "score": [3, 5, 4]})
    df.plot.bar(x="team", y="score", figsize=(8, 3))
    ax = df.plot.line(x="team", y="score")
    ax.set_ylabel("Score")
    df.set_index("team").plot.pie(y="score")
    return ax.get_ylabel()
