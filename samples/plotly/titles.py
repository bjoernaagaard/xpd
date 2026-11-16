"""Axis titles that px takes from a column name written as code.

A line that xpd reports ends with a comment that names the rule. Each case returns the
figures the code builds. This file sets no axis title on a figure afterwards, because the rule
is silent in a file that does.
"""

import plotly.express as px
import plotly.io as pio

pio.templates.default = "simple_white"

CASES = {}
COLUMN = "gdpPercap"


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def column_titles():
    gap = px.data.gapminder().query("year == 2007")
    tips = px.data.tips()
    camel = px.scatter(gap, x="gdpPercap", y="lifeExp", log_x=True)  # xpd: axis-title-column-name
    snake = px.histogram(tips, x="total_bill")  # xpd: axis-title-column-name
    on_y = px.box(tips, x="day", y="total_bill")  # xpd: axis-title-column-name
    labelled = px.scatter(gap, x="gdpPercap", y="lifeExp", log_x=True, labels={"gdpPercap": "GDP per person", "lifeExp": "Life expectancy"})
    words = px.bar(tips, x="day", y="tip")
    by_name = px.scatter(gap, x=COLUMN, y="pop")
    wide = px.line(gap.head(5), y=["lifeExp", "gdpPercap"])  # xpd: legend-instead-of-labels
    return camel, snake, on_y, labelled, words, by_name, wide
