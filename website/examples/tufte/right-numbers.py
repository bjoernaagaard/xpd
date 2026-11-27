import plotly.graph_objects as go


def load_data():
    return [(1978, 18), (1985, 27.5)]


def value_table(data):
    return {
        "columns": ["Deadline", "Standard (miles per gallon)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    x, y = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=x, y=y))
    fig.update_layout(template="simple_white", yaxis_title="Miles per gallon")
    return fig


def make_after(data) -> go.Figure:
    start, end = (row[1] for row in data)
    fig = go.Figure(
        go.Indicator(
            mode="number+delta",
            value=end,
            delta={"reference": start, "valueformat": ".1f", "suffix": " mpg"},
            number={"suffix": " mpg"},
            title={"text": "1985 standard and increase since 1978"},
        )
    )
    fig.update_layout(template="simple_white", height=250)
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Federal fuel economy standards reproduced in Tufte's book.",
    "question": "How much did the standard rise between the first and last deadlines?",
    "book": {"quote": "Finding the right numbers", "page": 83, "line": 3233},
    "before": {
        "caption": "Two bars show the endpoints. The difference requires subtraction.",
        "summary": "The standard rose from 18 to 27.5 miles per gallon, an increase of "
        "9.5 miles per gallon.",
    },
    "after": {
        "caption": "The display gives the 1985 standard and the increase since 1978 as numbers.",
        "summary": "The standard rose from 18 to 27.5 miles per gallon, an increase of "
        "9.5 miles per gallon.",
    },
}
