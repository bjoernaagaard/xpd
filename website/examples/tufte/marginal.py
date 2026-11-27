import plotly.express as px
import plotly.graph_objects as go


def load_data():
    iris = px.data.iris()
    return list(zip(iris["petal_length"], iris["petal_width"], strict=True))


def value_table(data):
    return {
        "columns": ["Petal length (cm)", "Petal width (cm)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    lengths, widths = zip(*data, strict=True)
    return px.scatter(
        x=lengths,
        y=widths,
        template="simple_white",
        labels={"x": "Petal length (cm)", "y": "Petal width (cm)"},
    )


def make_after(data) -> go.Figure:
    lengths, widths = zip(*data, strict=True)
    return px.scatter(
        x=lengths,
        y=widths,
        marginal_x="rug",
        marginal_y="rug",
        template="simple_white",
        labels={"x": "Petal length (cm)", "y": "Petal width (cm)"},
    )


EXAMPLE = {
    "source": "Plotly Express Iris dataset.",
    "question": "Where are the petal lengths and widths concentrated separately?",
    "book": {"quote": "the marginal distribution of each variable", "page": 135, "line": 5192},
    "before": {
        "caption": "The scatter shows joint clusters. Counting along either "
        "measurement separately requires tracing the points to an axis.",
        "summary": "The shortest band spans 1.0 to 1.9 cm in length and 0.1 to 0.6 cm "
        "in width. The other band starts at 3.0 cm in length and 1.0 cm in "
        "width.",
    },
    "after": {
        "caption": "Rugs at the top and right show gaps in each measurement. The petal "
        "lengths separate into two bands.",
        "summary": "The shortest band spans 1.0 to 1.9 cm in length and 0.1 to 0.6 cm "
        "in width. The other band starts at 3.0 cm in length and 1.0 cm in "
        "width.",
    },
}
