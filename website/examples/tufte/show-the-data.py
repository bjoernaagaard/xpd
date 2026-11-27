import numpy as np
import plotly.graph_objects as go
from plotly.subplots import make_subplots


def load_data() -> list[tuple[str, int, float]]:
    x = (10, 8, 13, 9, 11, 14, 6, 4, 12, 7, 5)
    series = (
        ("I", x, (8.04, 6.95, 7.58, 8.81, 8.33, 9.96, 7.24, 4.26, 10.84, 4.82, 5.68)),
        ("II", x, (9.14, 8.14, 8.74, 8.77, 9.26, 8.10, 6.13, 3.10, 9.13, 7.26, 4.74)),
        ("III", x, (7.46, 6.77, 12.74, 7.11, 7.81, 8.84, 6.08, 5.39, 8.15, 6.42, 5.73)),
        (
            "IV",
            (8, 8, 8, 8, 8, 8, 8, 19, 8, 8, 8),
            (6.58, 5.76, 7.71, 8.84, 8.47, 7.04, 5.25, 12.50, 5.56, 7.91, 6.89),
        ),
    )
    return [(name, a, b) for name, xs, ys in series for a, b in zip(xs, ys, strict=True)]


def value_table(data: list[tuple[str, int, float]]) -> dict:
    return {"columns": ["Series", "X", "Y"], "rows": [list(row) for row in data]}


def make_before(data: list[tuple[str, int, float]]) -> go.Figure:
    fig = make_subplots(
        rows=2, cols=2, subplot_titles=("I", "II", "III", "IV"), horizontal_spacing=0.15
    )
    for index, name in enumerate(("I", "II", "III", "IV")):
        points = sorted(
            ((x, y) for series, x, y in data if series == name), key=lambda pair: pair[0]
        )
        x, y = zip(*points, strict=True)
        fig.add_trace(
            go.Scatter(x=x, y=y, mode="lines", line_shape="spline", line_color="#245a80"),
            row=index // 2 + 1,
            col=index % 2 + 1,
        )
    fig.update_xaxes(range=[2, 21], matches="x")
    fig.update_xaxes(title_text="X", row=2)
    fig.update_yaxes(range=[2, 14], matches="y")
    fig.update_yaxes(title_text="Y", col=1)
    fig.update_layout(
        width=620,
        height=470,
        template="simple_white",
        showlegend=False,
        margin={"l": 42, "r": 16, "t": 34, "b": 40},
    )
    return fig


def make_after(data: list[tuple[str, int, float]]) -> go.Figure:
    fig = make_subplots(
        rows=2, cols=2, subplot_titles=("I", "II", "III", "IV"), horizontal_spacing=0.15
    )
    for index, name in enumerate(("I", "II", "III", "IV")):
        points = [(x, y) for series, x, y in data if series == name]
        x, y = zip(*points, strict=True)
        slope, intercept = np.polyfit(x, y, 1)
        ends = [min(x), max(x)]
        position = {"row": index // 2 + 1, "col": index % 2 + 1}
        fig.add_trace(
            go.Scatter(
                x=ends,
                y=[float(intercept + slope * end) for end in ends],
                mode="lines",
                name="Linear fit",
                line={"color": "#555555", "dash": "dash", "width": 1},
            ),
            **position,
        )
        fig.add_trace(
            go.Scatter(x=x, y=y, mode="markers", name="Observation", marker_color="#245a80"),
            **position,
        )
    fig.update_xaxes(range=[2, 21], matches="x")
    fig.update_xaxes(title_text="X", row=2)
    fig.update_yaxes(range=[2, 14], matches="y")
    fig.update_yaxes(title_text="Y", col=1)
    fig.update_layout(
        width=620,
        height=470,
        template="simple_white",
        showlegend=False,
        margin={"l": 42, "r": 16, "t": 34, "b": 40},
    )
    return fig


EXAMPLE = {
    "source": "F. J. Anscombe, Graphs in Statistical Analysis (1973), as reproduced by Tufte.",
    "question": "How do the four series differ when their fitted lines have nearly the same slope?",
    "book": {"quote": "show the data", "page": 13, "line": 108},
    "before": {
        "caption": (
            "Four smooth curves connect values of Y in order of X. The curve in series IV runs "
            "along X = 8 and then to X = 19, hiding which values were observed."
        ),
        "summary": (
            "Each series contains 11 pairs of unitless values. Series IV has ten observations at "
            "X = 8 and one at X = 19, where Y = 12.50."
        ),
    },
    "after": {
        "caption": (
            "Points show the observations and dashed lines show the linear fits on shared "
            "scales. Series II curves, series III has a high outlier, and one point in series IV "
            "determines the slope."
        ),
        "summary": (
            "All four fitted slopes round to 0.50. Series II reaches Y = 9.26 at X = 11. Series "
            "III has Y = 12.74 at X = 13. Series IV has ten points at X = 8 and one at X = 19."
        ),
    },
}
