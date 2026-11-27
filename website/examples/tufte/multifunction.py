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
    fig = go.Figure(go.Scatter(x=x, y=y, mode="lines+markers", name="Standard", showlegend=True))
    fig.update_layout(template="simple_white", yaxis_title="Miles per gallon")
    return fig


def make_after(data) -> go.Figure:
    x, y = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=x, y=y, mode="lines+markers", showlegend=False))
    fig.update_layout(
        template="simple_white", xaxis_title="Deadline", yaxis_title="Miles per gallon"
    )
    fig.update_xaxes(tickvals=[1978, 1982, 1985], showgrid=True, gridcolor="#e7e7e7")
    fig.update_yaxes(tickvals=[18, 24, 27.5])
    return fig


EXAMPLE = {
    "source": "Federal fuel economy standards reproduced in Tufte's book.",
    "question": "Which deadlines correspond to the lowest and highest standards?",
    "book": {"quote": "Mobilize every graphical element", "page": 140, "line": 5258},
    "before": {
        "caption": "The graph uses evenly spaced scale ticks and a separate legend for "
        "its single line.",
        "summary": "The first deadline called for 18 miles per gallon in 1978 and the "
        "last called for 27.5 in 1985.",
    },
    "after": {
        "caption": "Three ticks on each axis mark selected deadlines and standards. The "
        "lines from the deadline ticks lead to the observations.",
        "summary": "The first deadline called for 18 miles per gallon in 1978 and the "
        "last called for 27.5 in 1985.",
    },
}
