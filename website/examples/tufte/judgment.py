import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    days = ("Thur", "Fri", "Sat", "Sun")
    return [
        (day, meal, int(((tips["day"] == day) & (tips["time"] == meal)).sum()))
        for day in days
        for meal in ("Lunch", "Dinner")
    ]


def value_table(data):
    return {"columns": ["Day", "Meal", "Bills"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    from plotly.subplots import make_subplots

    fig = make_subplots(rows=1, cols=2, shared_yaxes=True, subplot_titles=("Lunch", "Dinner"))
    for column, meal in enumerate(("Lunch", "Dinner"), start=1):
        days, bills = zip(*((day, count) for day, time, count in data if time == meal), strict=True)
        fig.add_trace(go.Bar(x=days, y=bills, showlegend=False), row=1, col=column)
    fig.update_layout(template="simple_white")
    fig.update_yaxes(range=[0, 100], showgrid=False)
    return fig


def make_after(data) -> go.Figure:
    from plotly.subplots import make_subplots

    fig = make_subplots(rows=1, cols=2, shared_yaxes=True, subplot_titles=("Lunch", "Dinner"))
    for column, meal in enumerate(("Lunch", "Dinner"), start=1):
        days, bills = zip(*((day, count) for day, time, count in data if time == meal), strict=True)
        fig.add_trace(go.Bar(x=days, y=bills, showlegend=False), row=1, col=column)
    fig.update_layout(template="simple_white")
    fig.update_yaxes(range=[0, 100], dtick=20, showgrid=True, gridcolor="#e8e8e8", gridwidth=1)
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "How do dinner counts on Saturday and Sunday compare?",
    "book": {"quote": "Design is choice", "page": 191, "line": 8208},
    "before": {
        "caption": "The clean panels leave the reader to estimate bar heights across "
        "their full width.",
        "summary": "Saturday had 87 dinner bills and Sunday had 76.",
    },
    "after": {
        "caption": "A faint shared grid gives the bars a common reference. Saturday is "
        "above 80 and Sunday is below it.",
        "summary": "Saturday had 87 dinner bills and Sunday had 76.",
    },
}
