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
    fig = go.Figure(
        go.Table(
            header={"values": ["Group", "Share (%)"]},
            cells={"values": list(zip(*ranked, strict=True)), "align": ["left", "right"]},
        )
    )
    fig.update_layout(template="simple_white", height=280)
    return fig


EXAMPLE = {
    "source": "Illustrative rates for groups A, B and C come from the book. Group D is an additional illustrative value.",
    "question": "Which group has the highest rate?",
    "book": {"quote": "ordering by content or by data values", "page": 178, "line": 7149},
    "before": {
        "caption": "Alphabetical order separates the largest share from the next largest.",
        "summary": "The order is B at 35 percent, C at 29 percent, A at 24 percent and "
        "D at 12 percent.",
    },
    "after": {
        "caption": "The highest rate comes first, so the rows give the rank as well as the value.",
        "summary": "The order is B at 35 percent, C at 29 percent, A at 24 percent and "
        "D at 12 percent.",
    },
}
