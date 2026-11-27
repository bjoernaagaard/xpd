import plotly.graph_objects as go


def load_data():
    return [("A", 53), ("B", 46), ("C", 57), ("D", 49)]


def value_table(data):
    return {"columns": ["Group", "Share (%)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    fig = go.Figure(
        go.Table(
            header={"values": ["Group", "Share (%)"]},
            cells={"values": list(zip(*data, strict=True))},
        )
    )
    fig.update_layout(template="simple_white", height=290)
    return fig


def make_after(data) -> go.Figure:
    ranked = sorted(data, key=lambda row: row[1], reverse=True)
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
                "values": list(zip(*ranked, strict=True)),
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
        height=190,
        margin={"l": 0, "r": 0, "t": 12, "b": 8},
    )
    return fig


EXAMPLE = {
    "source": "Illustrative rates for groups A, B and C come from the book. Group D is an additional illustrative value.",
    "question": "Which group has the highest rate?",
    "book": {"quote": "ordering by content or by data values", "page": 178, "line": 7149},
    "before": {
        "caption": "Alphabetical order separates the largest share from the next largest.",
        "summary": "The order is C at 57 percent, A at 53 percent, D at 49 percent and "
        "B at 46 percent.",
    },
    "after": {
        "caption": "The highest rate comes first, so the rows give the rank as well as the value.",
        "summary": "The order is C at 57 percent, A at 53 percent, D at 49 percent and "
        "B at 46 percent.",
    },
}
