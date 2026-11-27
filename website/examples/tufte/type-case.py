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
    fig.update_layout(
        template="simple_white", yaxis_title="Median tip (USD)", title="MEDIAN TIPS BY DAY"
    )
    return fig


def make_after(data) -> go.Figure:
    days, tips = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=days, y=tips))
    fig.update_layout(
        template="simple_white", yaxis_title="Median tip (USD)", title="Median tips by day"
    )
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "Which day had the highest median tip?",
    "book": {"quote": "type is upper-and-lower case", "page": 183, "line": 7994},
    "before": {
        "caption": "The all-capital title dominates the four bars.",
        "summary": "The median tip was 3.15 dollars on Sunday and 3.00 dollars on Friday.",
    },
    "after": {
        "caption": "The sentence case title leaves the bars as the focus. Sunday has "
        "the highest median tip.",
        "summary": "The median tip was 3.15 dollars on Sunday and 3.00 dollars on Friday.",
    },
}
