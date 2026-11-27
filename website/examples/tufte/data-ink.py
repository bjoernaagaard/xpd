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
        template="plotly", xaxis_title="Sepal length (cm)", yaxis_title="Sepal width (cm)"
    )
    return fig


def make_after(data) -> go.Figure:
    x, y = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=x, y=y, mode="markers"))
    fig.update_layout(
        template="simple_white", xaxis_title="Sepal length (cm)", yaxis_title="Sepal width (cm)"
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Iris dataset.",
    "question": "Where are the iris sepal measurements concentrated?",
    "book": {"quote": "Maximize the data-ink ratio", "page": 98, "line": 3819},
    "before": {
        "caption": "A tinted plotting area and screen of grid lines occupy most of the chart.",
        "summary": "Sepal lengths range from 4.3 to 7.9 cm and widths range from 2.0 to 4.4 cm.",
    },
    "after": {
        "caption": "The plain background leaves the sepal measurements and their axes visible.",
        "summary": "Sepal lengths range from 4.3 to 7.9 cm and widths range from 2.0 to 4.4 cm.",
    },
}
