import plotly.graph_objects as go


def load_data():
    return [(1970, -11014), (1971, 397747), (1972, 521943), (1973, 1647001), (1974, 1435102)]


def value_table(data):
    return {"columns": ["Year", "Net income (nominal USD)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    years, values = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=years, y=values))
    fig.update_layout(
        template="simple_white",
        xaxis_title="Year",
        xaxis_tickvals=list(years),
        yaxis_title="Net income (USD)",
        yaxis_range=[200000, 1800000],
        yaxis_tickformat=",.0f",
    )
    return fig


def make_after(data) -> go.Figure:
    years, values = zip(*data, strict=True)
    colors = ["#dc4c3e" if value < 0 else "#3b82c4" for value in values]
    fig = go.Figure(
        go.Bar(
            x=years,
            y=values,
            marker_color=colors,
            hovertemplate="%{x}: $%{y:,.0f}<extra></extra>",
        )
    )
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        title={
            "text": "Day Mines net income, 1970 to 1974",
            "x": 0,
            "xanchor": "left",
            "subtitle": {"text": "Nominal USD with zero baseline preserved"},
        },
        xaxis_title="Year",
        xaxis_tickvals=list(years),
        yaxis_title="Net income (nominal USD)",
        yaxis_range=[-180000, 1850000],
        yaxis_tickprefix="$",
        yaxis_tickformat=",.0f",
        yaxis_zeroline=True,
        yaxis_zerolinecolor="#9ca3af",
        margin={"l": 80, "r": 24, "t": 84, "b": 64},
    )
    fig.add_annotation(
        x=1970,
        y=values[0],
        text="Loss of $11,014",
        ax=42,
        ay=-36,
        arrowhead=2,
        arrowwidth=1,
    )
    return fig


EXAMPLE = {
    "source": "Day Mines 1974 annual report as reproduced in Tufte's book.",
    "question": "Did Day Mines lose money in any of these years?",
    "book": {"quote": "the case of the disappearing baseline", "page": 58, "line": 1391},
    "before": {
        "caption": "The scale starts at 200,000 dollars. The 1970 loss is clipped and "
        "the visible profits look smaller.",
        "summary": "Day Mines lost 11,014 dollars in 1970 and earned 1,647,001 dollars in 1973.",
    },
    "after": {
        "caption": "The baseline at zero places the 1970 loss below the profits on the same scale.",
        "summary": "Day Mines lost 11,014 dollars in 1970 and earned 1,647,001 dollars in 1973.",
    },
}
