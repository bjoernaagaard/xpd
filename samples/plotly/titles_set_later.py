"""Axis titles from column names that the file replaces on the figure afterwards.

No line here is reported. Each case returns the figures the code builds.
"""

import plotly.express as px
import plotly.io as pio

pio.templates.default = "simple_white"

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def titled_later():
    gap = px.data.gapminder().query("year == 2007")
    fig = px.scatter(gap, x="gdpPercap", y="lifeExp", log_x=True)
    fig.update_layout(xaxis_title="GDP per person", yaxis_title="Life expectancy")
    return fig
