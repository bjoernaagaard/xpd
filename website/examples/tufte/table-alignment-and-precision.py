import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    medians = tips.groupby("day", observed=True)["total_bill"].median()
    return [(day, float(medians[day])) for day in ("Thur", "Fri", "Sat", "Sun")]


def value_table(data):
    return {"columns": ["Day", "Median bill (USD)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    fig = go.Figure(
        go.Table(
            header={"values": ["Day", "Median bill (USD)"]},
            cells={"values": list(zip(*data, strict=True)), "align": ["left", "left"]},
        )
    )
    fig.update_layout(template="simple_white", height=290)
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure(
        go.Table(
            header={"values": ["Day", "Median bill (USD)"]},
            cells={
                "values": list(zip(*data, strict=True)),
                "align": ["left", "right"],
                "format": ["", ".2f"],
            },
        )
    )
    fig.update_layout(template="simple_white", height=290)
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "How do median bills compare by day?",
    "book": {"quote": "Arrange the type to facilitate comparisons", "page": 178, "line": 7130},
    "before": {
        "caption": "The numeric cells align at the left and omit trailing zeros.",
        "summary": "Median bills were 16.20 dollars on Thursday, 15.38 on Friday, "
        "18.24 on Saturday and 19.63 on Sunday.",
    },
    "after": {
        "caption": "The numeric cells align at the right and show two decimal places so "
        "cents line up. Sunday has the highest median.",
        "summary": "Median bills were 16.20 dollars on Thursday, 15.38 on Friday, 18.24 "
        "on Saturday and 19.63 on Sunday.",
    },
}
