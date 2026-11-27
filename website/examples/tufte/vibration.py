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
    fig = go.Figure()
    for meal, color in (("Lunch", "#245a80"), ("Dinner", "#b98442")):
        days, counts = zip(
            *((day, count) for day, time, count in data if time == meal), strict=True
        )
        if meal == "Lunch":
            fig.add_trace(
                go.Bar(
                    x=days,
                    y=counts,
                    name=meal,
                    marker_color=color,
                    marker_pattern_shape="/",
                    marker_pattern_size=4,
                )
            )
        else:
            fig.add_trace(
                go.Bar(
                    x=days,
                    y=counts,
                    name=meal,
                    marker_color=color,
                    marker_pattern_shape="x",
                    marker_pattern_size=4,
                )
            )
    fig.update_layout(
        template="simple_white", barmode="group", yaxis_title="Bills", yaxis_range=[0, 105]
    )
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    for meal, color in (("Lunch", "#245a80"), ("Dinner", "#b98442")):
        days, counts = zip(
            *((day, count) for day, time, count in data if time == meal), strict=True
        )
        fig.add_trace(
            go.Bar(
                x=days,
                y=counts,
                name=meal,
                marker_color=color,
                text=[f"{meal} {count}" if count >= 10 else "" for count in counts],
                textposition="outside",
            )
        )
    fig.update_layout(
        template="simple_white", barmode="group", yaxis_title="Bills", yaxis_range=[0, 105]
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "Which days had the most lunch and dinner bills?",
    "book": {
        "quote": "statistical graphics are also often drawn up so as to shimmer",
        "page": 110,
        "line": 4092,
    },
    "before": {
        "caption": "Closely spaced hatching competes with the heights of the bars.",
        "summary": "Thursday had 61 lunch bills. Saturday had 87 dinner bills.",
    },
    "after": {
        "caption": "Flat fills and labels identify the meal counts. Thursday has the "
        "most lunches and Saturday the most dinners.",
        "summary": "Thursday had 61 lunch bills. Saturday had 87 dinner bills.",
    },
}
