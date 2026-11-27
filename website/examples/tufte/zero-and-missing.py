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
    fig = go.Figure(go.Bar(x=days, y=counts, name="Observed lunches"))
    fig.add_trace(
        go.Scatter(
            x=["Sun"],
            y=[0],
            mode="markers",
            marker={"symbol": "diamond", "size": 9},
            showlegend=False,
            cliponaxis=False,
        )
    )
    fig.add_annotation(x="Sun", y=0, text="Zero", yshift=14, showarrow=False)
    fig.add_annotation(x="Mon", y=0, text="Missing", yshift=14, showarrow=False)
    fig.update_layout(
        template="simple_white",
        yaxis_title="Lunch bills",
        xaxis_categoryorder="array",
        xaxis_categoryarray=[day for day, _ in data],
        xaxis_range=[-0.5, 4.5],
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
