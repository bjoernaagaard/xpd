import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    medians = tips.groupby("day", observed=True)["tip"].median()
    return [(day, float(medians[day])) for day in ("Thur", "Fri", "Sat", "Sun")]


def value_table(data):
    return {"columns": ["Day", "Median tip (USD)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(data, columns=["Day", "Median tip (USD)"])
    return px.bar(
        frame,
        x="Day",
        y="Median tip (USD)",
        color="Day",
        color_discrete_sequence=["#245a80", "#79465d", "#98732d", "#543f78"],
        template="simple_white",
    )


def make_after(data) -> go.Figure:
    days, tips = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=days, y=tips))
    fig.update_layout(template="simple_white", yaxis_title="Median tip (USD)")
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "How do median tips compare by day?",
    "book": {"quote": "Erase redundant data-ink", "page": 102, "line": 3877},
    "before": {
        "caption": "Every day gets its own color even though the day appears below the bar.",
        "summary": "The median tip was 2.305 dollars on Thursday, 3.00 on Friday, 2.75 "
        "on Saturday and 3.15 on Sunday.",
    },
    "after": {
        "caption": "One fill keeps the focus on bar height. Sunday has the highest median tip.",
        "summary": "The median tip was 2.305 dollars on Thursday, 3.00 on Friday, 2.75 "
        "on Saturday and 3.15 on Sunday.",
    },
}
