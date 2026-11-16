"""A file that draws through pandas and never says how its figures should look.

plotly then draws each figure with its default look, so xpd reports the statement that makes
plotly the plotting backend, once. Each case returns the figure the code builds.
"""

import pandas as pd

pd.options.plotting.backend = "plotly"  # xpd: default-template

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def scores_by_team():
    df = pd.DataFrame({"team": ["a", "b", "c"], "score": [3, 5, 4]})
    return df.plot.bar(x="team", y="score")
