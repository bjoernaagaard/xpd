import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    medians = tips.groupby("day", observed=True)["tip"].median()
    return [(day, float(medians[day])) for day in ("Thur", "Fri", "Sat", "Sun")]


def value_table(data):
    return {"columns": ["Day", "Median tip (USD)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    days, tips = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=days, y=tips))
    fig.update_layout(template="simple_white", yaxis_title="Median tip (USD)")
    fig.update_xaxes(showline=True, mirror=True)
    fig.update_yaxes(showline=True, mirror=True)
    return fig


def make_after(data) -> go.Figure:
    days, tips = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=days, y=tips))
    fig.update_layout(template="simple_white", yaxis_title="Median tip (USD)")
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "Which day had the highest median tip?",
    "book": {"quote": "The box can be erased", "page": 129, "line": 4954},
    "before": {
        "caption": "Mirrored axis lines surround the bars with a box.",
        "summary": "The median tip was 3.15 dollars on Sunday and 2.305 dollars on Thursday.",
    },
    "after": {
        "caption": "The value scale and day labels are enough to read the bars. Sunday "
        "has the highest median tip.",
        "summary": "The median tip was 3.15 dollars on Sunday and 2.305 dollars on Thursday.",
    },
}
