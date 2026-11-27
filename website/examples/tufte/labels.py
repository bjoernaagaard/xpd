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
    return go.Figure(go.Scatter(x=x, y=y, mode="lines+markers"))


def make_after(data) -> go.Figure:
    x, y = zip(*data, strict=True)
    fig = go.Figure(
        go.Scatter(
            x=x,
            y=y,
            mode="lines+markers",
            line={"width": 2, "color": "#3b82c4"},
            marker={"size": 7, "color": "#3b82c4"},
            hovertemplate="%{x}: %{y:.1f} mpg<extra></extra>",
        )
    )
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        title={
            "text": "Federal fuel economy standards",
            "x": 0,
            "xanchor": "left",
            "subtitle": {"text": "Mandated fleet average, 1978 to 1985"},
        },
        xaxis_title="Deadline",
        yaxis_title="Miles per gallon",
        yaxis_range=[16, 30],
        margin={"l": 64, "r": 32, "t": 84, "b": 64},
    )
    fig.add_annotation(x=1978, y=18, text="1978 deadline (18 mpg)", xanchor="left", ay=-28)
    fig.add_annotation(x=1985, y=27.5, text="1985 deadline (27.5 mpg)", xanchor="right", ay=28)
    return fig


EXAMPLE = {
    "source": "Federal fuel economy standards reproduced in Tufte's book.",
    "question": "When did the standards reach 18 and 27.5 miles per gallon?",
    "book": {"quote": "Clear, detailed, and thorough labeling", "page": 60, "line": 1527},
    "before": {
        "caption": "An unlabeled line rises over time. The vertical scale gives no unit.",
        "summary": "The standard rose from 18 miles per gallon in 1978 to 27.5 in 1985.",
    },
    "after": {
        "caption": "The vertical axis names the unit and the first and final deadlines "
        "are marked at the corresponding standards.",
        "summary": "The standard rose from 18 miles per gallon in 1978 to 27.5 in 1985.",
    },
}
