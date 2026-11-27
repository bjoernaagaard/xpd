# A plotly chart that passes the plotly pack and the chart review

A chart in the house style follows the plotly rules, which apply Edward Tufte's book "The Visual Display of Quantitative Information" to plotly figures, and answers the questions of the book that `xpd chart-review` asks for each chart. Layer a concise editorial `go.layout.Template` onto `"simple_white"` once, give titles a subtitle when context helps, label lines directly with range padding so labels never clip, format hover and tick values, and style `go.Table` with matching header and cell alignment and no heavy gray chrome.

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
"""A line chart of monthly revenue for each region, drawn with an editorial Tufte template.

The template layers typography, muted axis ink, an opaque white canvas (so dark ink stays readable
in dark-mode interactive windows; switch to transparent only when the host swaps text ink), and a
colorblind-safe colorway onto "simple_white". Each line carries its name at its last point with
explicit axis padding so labels never collide with axes or ticks.
"""

import pandas as pd
import plotly.graph_objects as go
import plotly.io as pio

TUFTE_COLORS = ["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"]

pio.templates["tufte"] = go.layout.Template(
    layout=go.Layout(
        colorway=TUFTE_COLORS,
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "size": 12, "color": "#1f2937"},
        title={"x": 0, "xanchor": "left", "font": {"size": 16}, "subtitle": {"font": {"size": 12, "color": "#4b5563"}}},
        paper_bgcolor="#ffffff",
        plot_bgcolor="#ffffff",
        hovermode="x unified",
        hoverlabel={"bgcolor": "#ffffff", "bordercolor": "#d1d5db", "font": {"color": "#1f2937"}},
        xaxis={"linecolor": "#6b7280", "tickcolor": "#6b7280", "automargin": True},
        yaxis={"linecolor": "#6b7280", "tickcolor": "#6b7280", "automargin": True},
    )
)
pio.templates.default = "simple_white+tufte"


def revenue_chart(revenue: pd.DataFrame) -> go.Figure:
    fig = go.Figure()
    dashes = ("solid", "dash", "dot", "dashdot")
    for idx, (region, months) in enumerate(revenue.sort_values("month").groupby("region")):
        color = TUFTE_COLORS[idx % len(TUFTE_COLORS)]
        trace = go.Scatter(
            x=months["month"],
            y=months["revenue"],
            name=str(region),
            mode="lines",
            line={"width": 2, "color": color, "dash": dashes[idx % len(dashes)]},
            hovertemplate="%{y:.1f}M EUR<extra>%{fullData.name}</extra>",
        )
        fig.add_trace(trace)
        fig.add_annotation(
            x=months["month"].iloc[-1],
            y=float(months["revenue"].iloc[-1]),
            text=str(region),
            showarrow=False,
            xanchor="left",
            xshift=6,
            font={"color": color, "size": 12},
        )
    ymax = float(revenue["revenue"].max())
    return fig.update_layout(
        title={
            "text": "Monthly revenue by region",
            "subtitle": {"text": "Constant 2024 euro (millions)"},
        },
        xaxis_title="Month",
        yaxis_title="Revenue (million 2024 EUR)",
        yaxis_range=[0, ymax * 1.12],
        yaxis_tickformat=".0f",
        showlegend=False,
        margin={"l": 64, "r": 84, "t": 68, "b": 48},
    )


def summary_table(rows: list[tuple[str, float, float]]) -> go.Figure:
    """Minimal Tufte go.Table with matching header and cell alignment and no gray chrome."""
    align = ["left", "right", "right"]
    table = go.Table(
        header={
            "values": ["Region", "Median revenue (M EUR)", "Orders"],
            "align": align,
            "fill": {"color": "rgba(15,23,42,0.04)"},
            "line": {"color": "#cbd5e1", "width": 1},
            "font": {"color": "#1f2937", "size": 12},
            "height": 32,
        },
        cells={
            "values": list(zip(*rows, strict=True)),
            "align": align,
            "format": ["", ".2f", ",d"],
            "fill": {"color": "#ffffff"},
            "line": {"width": 0},
            "font": {"color": "#1f2937", "size": 12},
            "height": 28,
        },
    )
    return go.Figure(table).update_layout(
        height=32 + 28 * len(rows) + 40,
        margin={"l": 0, "r": 0, "t": 16, "b": 8},
    )
```

## Commands

1. `xpd chart-review <file> --format json` lists each figure in the file with its mechanical Tufte findings and per-figure checklist across all 57 principles of the book.
2. `xpd chart-review <file> --adapter <adapter.py> --format json` renders the figures that the adapter's `figures()` returns in the project's Python and checks all 10 runtime facts (canvas, values, colors, x-values, text, axes, banking, table-borders, table-numbers, and table-structure).
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
