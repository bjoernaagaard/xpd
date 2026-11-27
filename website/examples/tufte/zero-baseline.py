import plotly.graph_objects as go


def load_data():
    return [(1970, -11014), (1971, 397747), (1972, 521943), (1973, 1647001), (1974, 1435102)]


def value_table(data):
    return {"columns": ["Year", "Net income (USD)"], "rows": [list(row) for row in data]}


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
    fig = go.Figure(go.Bar(x=years, y=values))
    fig.update_layout(
        template="simple_white",
        xaxis_title="Year",
        xaxis_tickvals=list(years),
        yaxis_title="Net income (USD)",
        yaxis_range=[-150000, 1800000],
        yaxis_tickformat=",.0f",
    )
    fig.add_annotation(
        x=1970,
        y=values[0],
        text="Loss of $11,014",
        ax=38,
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
