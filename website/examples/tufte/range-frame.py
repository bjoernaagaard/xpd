import plotly.express as px
import plotly.graph_objects as go


def load_data():
    iris = px.data.iris()
    return list(zip(iris["sepal_length"], iris["sepal_width"], strict=True))


def value_table(data):
    return {
        "columns": ["Sepal length (cm)", "Sepal width (cm)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    x, y = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=x, y=y, mode="markers"))
    fig.update_layout(
        template="simple_white",
        xaxis_range=[0, 10],
        yaxis_range=[0, 10],
        xaxis_title="Sepal length (cm)",
        yaxis_title="Sepal width (cm)",
    )
    return fig


def make_after(data) -> go.Figure:
    x, y = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=x, y=y, mode="markers"))
    fig.update_layout(
        template="simple_white", xaxis_title="Sepal length (cm)", yaxis_title="Sepal width (cm)"
    )
    fig.update_xaxes(range=[min(x), max(x)], tickvals=[min(x), 5, 6, 7, max(x)])
    fig.update_yaxes(range=[min(y), max(y)], tickvals=[min(y), 3, 4, max(y)])
    fig.update_traces(cliponaxis=False)
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Iris dataset.",
    "question": "What range of sepal lengths and widths did the measurements cover?",
    "book": {"quote": "an effective data-communicating element", "page": 132, "line": 5156},
    "before": {
        "caption": "Both axes extend from 0 to 10 cm, leaving much of the plotting area empty.",
        "summary": "Sepal lengths range from 4.3 to 7.9 cm and widths range from 2.0 to 4.4 cm.",
    },
    "after": {
        "caption": "The axis endpoints mark the smallest and largest observed length and width.",
        "summary": "Sepal lengths range from 4.3 to 7.9 cm and widths range from 2.0 to 4.4 cm.",
    },
}
