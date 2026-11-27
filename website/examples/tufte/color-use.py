import numpy as np
import plotly.express as px
import plotly.graph_objects as go


def load_data():
    iris = px.data.iris()
    counts, _, _ = np.histogram2d(
        iris["sepal_width"], iris["sepal_length"], bins=([2, 2.5, 3, 3.5, 4, 4.5], [4, 5, 6, 7, 8])
    )
    return [
        (f"{length} to {length + 1}", f"{width:g} to {width + 0.5:g}", int(counts[row, col]))
        for row, width in enumerate((2, 2.5, 3, 3.5, 4))
        for col, length in enumerate(range(4, 8))
    ]


def value_table(data):
    return {
        "columns": ["Length band (cm)", "Width band (cm)", "Flowers"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    lengths = list(dict.fromkeys(length for length, _, _ in data))
    widths = list(dict.fromkeys(width for _, width, _ in data))
    counts = [[count for _, band, count in data if band == width] for width in widths]
    fig = go.Figure(
        go.Heatmap(
            x=lengths, y=widths, z=counts, colorscale="Rainbow", colorbar={"title": "Flowers"}
        )
    )
    fig.update_layout(
        template="simple_white", xaxis_title="Sepal length (cm)", yaxis_title="Sepal width (cm)"
    )
    return fig


def make_after(data) -> go.Figure:
    lengths = list(dict.fromkeys(length for length, _, _ in data))
    widths = list(dict.fromkeys(width for _, width, _ in data))
    counts = [[count for _, band, count in data if band == width] for width in widths]
    fig = go.Figure(
        go.Heatmap(x=lengths, y=widths, z=counts, colorscale="Blues", colorbar={"title": "Flowers"})
    )
    fig.update_layout(
        template="simple_white", xaxis_title="Sepal length (cm)", yaxis_title="Sepal width (cm)"
    )
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Iris dataset, binned into half-open intervals.",
    "question": "Which sepal measurement bands contain the most flowers?",
    "book": {"quote": "Color often generates graphical puzzles", "page": 155, "line": 5939},
    "before": {
        "caption": "Unrelated hues represent counts. Their order is hard to infer "
        "without the color bar.",
        "summary": "The table gives the counts for all 20 bins, which together contain "
        "150 flowers.",
    },
    "after": {
        "caption": "Increasing blue darkness follows increasing counts. The middle "
        "length and width bands contain many of the flowers.",
        "summary": "The table gives the counts for all 20 bins, which together contain "
        "150 flowers.",
    },
}
