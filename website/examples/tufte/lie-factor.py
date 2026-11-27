import plotly.graph_objects as go


def load_data():
    return [(1978, 18), (1985, 27.5)]


def value_table(data):
    return {
        "columns": ["Deadline", "Standard (miles per gallon)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    years, values = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=years, y=values))
    fig.update_layout(template="simple_white", yaxis_title="Miles per gallon", yaxis_range=[16, 30])
    return fig


def make_after(data) -> go.Figure:
    years, values = zip(*data, strict=True)
    fig = go.Figure(
        go.Bar(
            x=years,
            y=values,
            marker_color="#3b82c4",
            text=[f"{v:g} mpg" for v in values],
            textposition="outside",
            cliponaxis=False,
            hovertemplate="%{x}: %{y:g} mpg<extra></extra>",
        )
    )
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        title={
            "text": "Federal fuel economy standard, 1978 and 1985",
            "x": 0,
            "xanchor": "left",
            "subtitle": {"text": "Zero baseline keeps bar lengths proportional to the 53% rise"},
        },
        xaxis_title="Deadline",
        xaxis_tickvals=list(years),
        yaxis_title="Miles per gallon",
        yaxis_range=[0, 32],
        margin={"l": 60, "r": 24, "t": 84, "b": 64},
    )
    return fig


EXAMPLE = {
    "source": "Federal fuel economy standards reproduced in Tufte's book.",
    "question": "How large was the increase in the fuel economy standard?",
    "book": {"quote": "the graphic might be doing a reasonable job", "page": 61, "line": 1535},
    "before": {
        "caption": "The scale starts at 16. The visible bar lengths of 2 and 11.5 "
        "imply a 475 percent increase.",
        "summary": "The standard rose from 18 to 27.5 miles per gallon, an increase of "
        "9.5 miles per gallon.",
    },
    "after": {
        "caption": "The bars start at zero. Their lengths increase by about 53 percent, "
        "the same as the standards.",
        "summary": "The standard rose from 18 to 27.5 miles per gallon, an increase of "
        "9.5 miles per gallon.",
    },
}
