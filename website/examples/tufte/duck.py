import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    medians = tips.groupby("day", observed=True)["total_bill"].median()
    return [(day, float(medians[day])) for day in ("Thur", "Fri", "Sat", "Sun")]


def value_table(data):
    return {"columns": ["Day", "Median bill (USD)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    fig = go.Figure()
    for index, (day, bill) in enumerate(data):
        fig.add_trace(
            go.Indicator(
                mode="gauge+number",
                value=bill,
                title={"text": day},
                gauge={"axis": {"range": [0, 25]}},
                domain={"row": index // 2, "column": index % 2},
            )
        )
    fig.update_layout(template="simple_white", grid={"rows": 2, "columns": 2}, height=450)
    return fig


def make_after(data) -> go.Figure:
    days, bills = zip(*data, strict=True)
    fig = go.Figure(
        go.Bar(x=days, y=bills, text=bills, textposition="outside", texttemplate="%{text:.2f}")
    )
    fig.update_layout(template="simple_white", yaxis_range=[0, 24], yaxis_title="Median bill (USD)")
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "Which day had the highest median bill?",
    "book": {
        "quote": "When a graphic is taken over by decorative forms",
        "page": 118,
        "line": 4588,
    },
    "before": {
        "caption": "Four separate dials repeat the scale and make the reader compare "
        "needle positions.",
        "summary": "Median bills were 15.38 dollars on Friday and 19.63 on Sunday.",
    },
    "after": {
        "caption": "Four bars share a baseline and show the values above their ends. "
        "Sunday has the highest median.",
        "summary": "Median bills were 15.38 dollars on Friday and 19.63 on Sunday.",
    },
}
