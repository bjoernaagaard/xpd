import plotly.graph_objects as go


def load_data():
    return [("A", 53), ("B", 46), ("C", 57)]


def value_table(data):
    return {"columns": ["Group", "Share (%)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    fig = go.Figure(
        go.Scatter(x=[0], y=[0], mode="text", text=["A has 53%, B has 46% and C has 57%."])
    )
    fig.update_layout(template="simple_white", height=220, xaxis_visible=False, yaxis_visible=False)
    return fig


def make_after(data) -> go.Figure:
    align = ["left", "right"]
    fig = go.Figure(
        go.Table(
            header={
                "values": ["Group", "Share (%)"],
                "align": align,
                "fill": {"color": "rgba(15,23,42,0.04)"},
                "line": {"color": "#cbd5e1", "width": 1},
                "font": {"color": "#1f2937", "size": 12},
                "height": 34,
            },
            cells={
                "values": list(zip(*data, strict=True)),
                "align": align,
                "fill": {"color": "rgba(0,0,0,0)"},
                "line": {"width": 0},
                "font": {"color": "#1f2937", "size": 12},
                "height": 30,
            },
        )
    )
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        height=160,
        margin={"l": 0, "r": 0, "t": 12, "b": 8},
    )
    return fig


EXAMPLE = {
    "source": "Illustrative group rates from the book.",
    "question": "How do the three group shares compare?",
    "book": {
        "quote": "The conventional sentence is a poor way to show more than two numbers",
        "page": 178,
        "line": 7126,
    },
    "before": {
        "caption": "A sentence gives the shares in reading order, which makes it "
        "harder to scan the numbers.",
        "summary": "Group A has 53 percent, B has 46 percent and C has 57 percent.",
    },
    "after": {
        "caption": "One row per group lines up the shares. C has the largest share.",
        "summary": "Group A has 53 percent, B has 46 percent and C has 57 percent.",
    },
}
