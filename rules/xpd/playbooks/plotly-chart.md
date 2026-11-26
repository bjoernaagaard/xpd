# A plotly chart that passes the plotly pack and the chart review

A chart in the house style follows the plotly rules, which apply Edward Tufte's book "The Visual Display of Quantitative Information" to plotly figures, and answers the questions of the book that `xpd chart-review` asks for each chart. The template is plain and set once, each line carries its own label in place of a legend, the axes carry words and a unit, and the value axis starts at zero. A chart in matplotlib, seaborn or another library gets the same review, with the plotly call that draws it.

## Asks

- improve the charts I am generating
- improve my charts
- make my plotly figure better
- implement a plotly chart from scratch based on our ruleset derived from the book
- make a chart, a plot or a graph that follows the plotly rules
- review the charts of my dashboard against Tufte's book
- review a matplotlib or seaborn figure and redraw it in plotly

## Code

```python
"""A line chart of monthly revenue for each region, drawn the way the plotly pack asks.

The template is set once for every figure of the program. Each line carries its name at its last
point, so the figure needs no legend, and the axes carry words and a unit.
"""

import pandas as pd
import plotly.graph_objects as go
import plotly.io as pio

pio.templates.default = "simple_white"


def region_line(region: str, months: pd.DataFrame) -> go.Scatter:
    labels = [""] * (len(months) - 1) + [region]
    return go.Scatter(
        x=months["month"],
        y=months["revenue"],
        mode="lines+text",
        text=labels,
        textposition="middle right",
        line_color="black",
    )


def revenue_chart(revenue: pd.DataFrame) -> go.Figure:
    regions = revenue.sort_values("month").groupby("region")
    lines = [region_line(str(region), months) for region, months in regions]
    return go.Figure(data=lines).update_layout(
        title_text="Monthly revenue by region",
        xaxis_title="Month",
        yaxis_title="Revenue in million euro",
        yaxis_rangemode="tozero",
        showlegend=False,
    )
```

## Commands

1. `xpd chart-review <file> --format json` lists each figure in the file with its mechanical Tufte findings and per-figure checklist across all 57 principles of the book.
2. `xpd chart-review <file> --adapter <adapter.py> --format json` renders the figures that the adapter's `figures()` returns in the project's Python and checks all 10 runtime facts (canvas, values, colors, x-values, text, slopes, axes, legend, table, and grid).
3. `xpd chart-review start <paths> --check "<tests>" --adapter <adapter.py>` starts a recorded review ledger so `plan`, `gate`, and `finish` verify every fix and every checklist item.
4. `xpd rules --rule plotly --format json` gives the documentation of each plotly rule, with the page of the book, the plotly code to write and the questions of the review that the rule reports.
5. `xpd find "<chart form>" --kind rule` names the rule for a form, such as "small multiples" or "pie chart".

## Rules

- plotly/default-template
- plotly/legend-instead-of-labels
- plotly/axis-title-column-name
- plotly/bar-axis-without-zero
- plotly/area-axis-without-zero
- plotly/explicit-grid
- plotly/boxed-axis-frame
- plotly/redundant-color
- plotly/rainbow-colorscale
- plotly/pie-chart
- plotly/rotated-tick-labels
- plotly/all-caps-title
