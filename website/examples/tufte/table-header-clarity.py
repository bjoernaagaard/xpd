import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    summary = tips.groupby("day", observed=True).agg(
        total_bill=("total_bill", "median"), party_size=("size", "mean")
    )
    return [
        (
            day,
            float(summary.loc[day, "total_bill"]),
            round(float(summary.loc[day, "party_size"]), 2),
        )
        for day in ("Thur", "Fri", "Sat", "Sun")
    ]


def value_table(data):
    return {
        "columns": ["Day", "Median bill (USD)", "Mean people per party"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    fig = go.Figure(
        go.Table(
            header={"values": ["day", "total_bill", "party_size"]},
            cells={"values": list(zip(*data, strict=True))},
        )
    )
    fig.update_layout(template="simple_white", height=290)
    return fig


def make_after(data) -> go.Figure:
    align = ["left", "right", "right"]
    fig = go.Figure(
        go.Table(
            header={
                "values": ["Day", "Median bill (USD)", "Mean people per party"],
                "align": align,
                "fill": {"color": "rgba(15,23,42,0.04)"},
                "line": {"color": "#cbd5e1", "width": 1},
                "font": {"color": "#1f2937", "size": 12},
                "height": 34,
            },
            cells={
                "values": list(zip(*data, strict=True)),
                "align": align,
                "format": ["", ".2f", ".2f"],
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
    "source": "Plotly Express Tips dataset.",
    "question": "How do the typical bill and average party size vary by day?",
    "book": {"quote": "words are spelled out", "page": 183, "line": 7966},
    "before": {
        "caption": "The table headings use column identifiers and give no units or summary method.",
        "summary": "Sunday had a median bill of 19.63 dollars and an average of about "
        "2.84 people per party.",
    },
    "after": {
        "caption": "The headings state the units, match their column alignment, and distinguish "
        "a median bill from a mean party size.",
        "summary": "Sunday had a median bill of 19.63 dollars and an average of about "
        "2.84 people per party.",
    },
}
