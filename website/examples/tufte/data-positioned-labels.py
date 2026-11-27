import plotly.graph_objects as go


def load_data():
    return [
        (1978, 18),
        (1979, 19),
        (1980, 20),
        (1981, 22),
        (1982, 24),
        (1983, 26),
        (1984, 27),
        (1985, 27.5),
    ]


def value_table(data):
    return {
        "columns": ["Deadline", "Standard (miles per gallon)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    x, y = zip(*data, strict=True)
    fig = go.Figure(
        go.Scatter(x=x, y=y, mode="lines+markers", name="Fuel economy", showlegend=True)
    )
    fig.update_layout(template="simple_white", yaxis_title="Miles per gallon")
    return fig


def make_after(data) -> go.Figure:
    x, y = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=x, y=y, mode="lines+markers", showlegend=False))
    fig.update_layout(template="simple_white", yaxis_title="Miles per gallon", yaxis_range=[16, 30])
    for year, standard in data:
        fig.add_annotation(x=year, y=standard, text=f"{standard:g}", showarrow=False, yshift=13)
    return fig


EXAMPLE = {
    "source": "Federal fuel economy standards reproduced in Tufte's book.",
    "question": "What were the exact fuel economy standards?",
    "book": {
        "quote": "the coordinate labels are far from the data measures",
        "page": 151,
        "line": 5768,
    },
    "before": {
        "caption": "The line shows rising standards. A reader has to estimate each "
        "value against the axis.",
        "summary": "The targets were 18 miles per gallon in 1978, 24 in 1982 and 27.5 in 1985.",
    },
    "after": {
        "caption": "The value sits above each point. The final label gives the "
        "fractional target of 27.5 miles per gallon.",
        "summary": "The targets were 18 miles per gallon in 1978, 24 in 1982 and 27.5 in 1985.",
    },
}
