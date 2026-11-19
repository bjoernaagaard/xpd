# A plotly chart that passes the plotly pack

A chart in the house style follows the plotly rules, which apply Edward Tufte's book "The Visual Display of Quantitative Information" to plotly figures. The template is plain and set once, each line carries its own label in place of a legend, the axes carry words and a unit, and the value axis starts at zero.

## Asks

- improve the charts I am generating
- improve my charts
- make my plotly figure better
- implement a plotly chart from scratch based on our ruleset derived from the book
- make a chart, a plot or a graph that follows the plotly rules

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

1. `xpd check <file> --format json` reports each form of the figure that the plotly pack argues against.
2. `xpd rules --rule plotly --format json` gives the documentation of each plotly rule, with the page of the book and the plotly code to write.
3. `xpd find "<chart form>" --kind rule` names the rule for a form, such as "small multiples" or "pie chart".
4. `xpd check <file> --format json` after the change, and the chart is done when it reports 0 issues.

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
