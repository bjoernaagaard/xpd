import plotly.graph_objects as go


def load_data():
    return [("A", 53), ("B", 46), ("C", 57)]


def value_table(data):
    return {"columns": ["Group", "Share (%)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    fig = go.Figure(go.Bar(x=["A", "B", "C"], y=[53, 46, 57], marker_color="#3b82c4"))
    fig.update_layout(template="simple_white", yaxis_title="Share (%)")
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
    "question": "What is the exact share of each group?",
    "book": {
        "quote": "small, noncomparative, highly labeled data sets usually belong in tables",
        "page": 37,
        "line": 653,
    },
    "before": {
        "caption": "Three bars show that C has the largest share. The bars require a "
        "scale lookup for exact shares.",
        "summary": "Group A has 53 percent, B has 46 percent and C has 57 percent.",
    },
    "after": {
        "caption": "The table puts each share next to its group. C has the largest share.",
        "summary": "Group A has 53 percent, B has 46 percent and C has 57 percent.",
    },
}
