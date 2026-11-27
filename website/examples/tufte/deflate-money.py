import plotly.graph_objects as go


def load_data():
    return [(1967, 310, 310), (1977, round(380 * 2.03), 380)]


def value_table(data):
    return {
        "columns": ["Year", "Nominal dollars per resident", "Constant dollars per resident"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    years, nominal, _ = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=years, y=nominal, mode="lines+markers"))
    fig.update_layout(
        template="simple_white",
        xaxis_tickvals=list(years),
        yaxis_title="Budget per resident (USD)",
        yaxis_range=[250, 825],
    )
    return fig


def make_after(data) -> go.Figure:
    years, _, real = zip(*data, strict=True)
    fig = go.Figure(
        go.Scatter(
            x=years,
            y=real,
            mode="lines+markers+text",
            text=[f"${v:,.0f}" for v in real],
            textposition="top center",
            line={"width": 2, "color": "#3b82c4"},
            marker={"size": 8, "color": "#3b82c4"},
            hovertemplate="%{x}: $%{y:,.0f} (1967 USD)<extra></extra>",
        )
    )
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        title={
            "text": "New York State budget per resident",
            "x": 0,
            "xanchor": "left",
            "subtitle": {"text": "Deflated to constant 1967 dollars"},
        },
        xaxis_title="Year",
        xaxis_tickvals=list(years),
        yaxis_title="Budget per resident (constant 1967 USD)",
        yaxis_tickprefix="$",
        yaxis_range=[250, 825],
        margin={"l": 72, "r": 32, "t": 84, "b": 64},
    )
    return fig


EXAMPLE = {
    "source": "New York State budget endpoints from the graph reproduced by Tufte. Nominal "
    "1977 dollars are inferred using the book's price factor of 2.03.",
    "question": "How much did New York State spending per person grow after inflation?",
    "book": {
        "quote": "deflated and standardized units of monetary measurement",
        "page": 72,
        "line": 2635,
    },
    "before": {
        "caption": "The axis leaves the price basis unspecified. The unadjusted "
        "endpoints suggest that spending per resident more than doubled.",
        "summary": "In 1967 the budget was about 310 dollars per resident. In 1977 it "
        "was about 771 nominal dollars or 380 in 1967 dollars.",
    },
    "after": {
        "caption": "The endpoints use constant 1967 dollars on the same scale. The "
        "change after inflation is much smaller.",
        "summary": "In 1967 the budget was about 310 dollars per resident. In 1977 it "
        "was about 771 nominal dollars or 380 in 1967 dollars.",
    },
}
