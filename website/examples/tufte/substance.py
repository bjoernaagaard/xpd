import plotly.express as px
import plotly.graph_objects as go


def load_data() -> list[tuple[str, float, float]]:
    iris = px.data.iris()
    return list(zip(iris["species"], iris["petal_length"], iris["petal_width"], strict=True))


def value_table(data: list[tuple[str, float, float]]) -> dict:
    return {
        "columns": ["Species", "Petal length (cm)", "Petal width (cm)"],
        "rows": [list(row) for row in data],
    }


def make_before(data: list[tuple[str, float, float]]) -> go.Figure:
    fig = go.Figure()
    for species, symbol in (
        ("setosa", "circle"),
        ("versicolor", "square"),
        ("virginica", "diamond"),
    ):
        x, y = zip(
            *((length, width) for name, length, width in data if name == species), strict=True
        )
        fig.add_trace(
            go.Scatter(
                x=x,
                y=y,
                name=species,
                mode="markers",
                marker={"color": "#245a80", "symbol": symbol, "size": 8},
            )
        )
    fig.update_layout(
        width=620,
        height=430,
        template="simple_white",
        title="Scatter method",
        xaxis_title="Petal length (cm)",
        yaxis_title="Petal width (cm)",
        plot_bgcolor="#f0c9a7",
        paper_bgcolor="#ffe6c9",
        legend={"orientation": "h", "y": 1.02, "yanchor": "bottom"},
        margin={"l": 52, "r": 16, "t": 82, "b": 58},
        annotations=[
            {
                "text": "150 projected marker glyphs",
                "xref": "paper",
                "yref": "paper",
                "x": 0.57,
                "y": 0.68,
                "showarrow": False,
                "bgcolor": "#ffffff",
                "bordercolor": "#663d25",
                "borderwidth": 3,
                "borderpad": 12,
            },
        ],
    )
    fig.update_xaxes(showgrid=True, gridwidth=2, gridcolor="#a46e53")
    fig.update_yaxes(showgrid=True, gridwidth=2, gridcolor="#a46e53")
    return fig


def make_after(data: list[tuple[str, float, float]]) -> go.Figure:
    fig = go.Figure()
    for species, symbol in (
        ("setosa", "circle"),
        ("versicolor", "square"),
        ("virginica", "diamond"),
    ):
        x, y = zip(
            *((length, width) for name, length, width in data if name == species), strict=True
        )
        fig.add_trace(
            go.Scatter(
                x=x,
                y=y,
                name=species,
                mode="markers",
                marker={"color": "#245a80", "symbol": symbol, "size": 8},
            )
        )
    fig.update_layout(
        width=620,
        height=430,
        template="simple_white",
        title="Iris petals by species",
        xaxis_title="Petal length (cm)",
        yaxis_title="Petal width (cm)",
        legend={"orientation": "h", "y": 1.02, "yanchor": "bottom"},
        margin={"l": 52, "r": 16, "t": 82, "b": 58},
    )
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Iris dataset, with 50 flowers from each of three species.",
    "question": "Which iris species can you distinguish by petal length and width?",
    "book": {
        "quote": "induce the viewer to think about the substance",
        "page": 13,
        "line": 109,
    },
    "before": {
        "caption": (
            "The chart uses a tinted background, heavy grid and a large note about how the "
            "scatter was drawn. The note crosses the area where versicolor and virginica "
            "measurements meet."
        ),
        "summary": (
            "The 150 flowers include setosa petals from 1.0 to 1.9 cm long and versicolor petals "
            "from 3.0 to 5.1 cm long."
        ),
    },
    "after": {
        "caption": (
            "Marker shapes identify the species on a plain background. Setosa forms a separate "
            "cluster of short, narrow petals. Versicolor and virginica overlap."
        ),
        "summary": (
            "Versicolor petals measure 3.0 to 5.1 cm long and 1.0 to 1.8 cm wide. Virginica "
            "petals measure 4.5 to 6.9 cm long and 1.4 to 2.5 cm wide."
        ),
    },
}
