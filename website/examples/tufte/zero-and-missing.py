import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    return [
        (day, int(((tips["day"] == day) & (tips["time"] == "Lunch")).sum()))
        for day in ("Thur", "Fri", "Sat", "Sun")
    ] + [("Mon", None)]


def value_table(data):
    return {"columns": ["Day", "Lunch bills"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    days, counts = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=days, y=counts))
    fig.update_layout(template="simple_white", yaxis_title="Lunch bills")
    return fig


def make_after(data) -> go.Figure:
    days, counts = zip(*((day, count) for day, count in data if count), strict=True)
    fig = go.Figure(
        go.Bar(
            x=days,
            y=counts,
            name="Observed lunches",
            marker_color="#3b82c4",
            hovertemplate="%{x}: %{y} lunch bills<extra></extra>",
        )
    )
    fig.add_trace(
        go.Scatter(
            x=["Sun"],
            y=[0],
            mode="markers",
            marker={"symbol": "diamond", "size": 9, "color": "#d97706"},
            showlegend=False,
            cliponaxis=False,
            hovertemplate="Sun: 0 observed lunch bills<extra></extra>",
        )
    )
    fig.add_annotation(x="Sun", y=0, text="Zero", yshift=16, showarrow=False)
    fig.add_annotation(x="Mon", y=0, text="Missing", yshift=16, showarrow=False)
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        title={
            "text": "Lunch bills by day of week",
            "x": 0,
            "xanchor": "left",
            "subtitle": {"text": "Sunday has zero observed lunches; Monday was not recorded"},
        },
        xaxis_title="Day",
        yaxis_title="Lunch bills",
        yaxis_range=[0, 72],
        xaxis_categoryorder="array",
        xaxis_categoryarray=[day for day, _ in data],
        xaxis_range=[-0.5, 4.5],
        margin={"l": 60, "r": 24, "t": 84, "b": 64},
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset. Monday has no observations.",
    "question": "How do zero lunch bills and a missing day differ?",
    "book": {"quote": "the plain dots register a zero reading", "page": 146, "line": 5483},
    "before": {
        "caption": "The bars leave empty spaces for both Sunday and Monday, although "
        "the reasons differ.",
        "summary": "Thursday had 61 lunch bills and Sunday had zero. No count is "
        "available for Monday.",
    },
    "after": {
        "caption": "A diamond marks an observed zero on Sunday. Monday has a label for "
        "a missing observation.",
        "summary": "Thursday had 61 lunch bills and Sunday had zero. No count is "
        "available for Monday.",
    },
}
