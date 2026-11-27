import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    return [
        (str(day), float(tip))
        for day, tip in tips[["day", "tip"]].itertuples(index=False, name=None)
    ]


def value_table(data):
    return {"columns": ["Day", "Tip (USD)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    totals = {
        day: sum(tip for name, tip in data if name == day) for day in ("Thur", "Fri", "Sat", "Sun")
    }
    fig = go.Figure(go.Bar(x=list(totals), y=list(totals.values())))
    fig.update_layout(template="simple_white", yaxis_title="Total tips (USD)")
    return fig


def make_after(data) -> go.Figure:
    days, tips = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=days, y=tips, mode="markers", marker={"opacity": 0.45}))
    fig.update_layout(
        template="simple_white",
        yaxis_title="Individual tip (USD)",
        xaxis_categoryorder="array",
        xaxis_categoryarray=["Thur", "Fri", "Sat", "Sun"],
    )
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "How widely did individual tips vary on each day?",
    "book": {"quote": "serve a reasonably clear purpose", "page": 17, "line": 115},
    "before": {
        "caption": "Daily sums answer how much was collected. They hide the spread of "
        "individual tips.",
        "summary": "Saturday tips ranged from 1.00 to 10.00 dollars. Friday tips "
        "ranged from 1.00 to 4.73 dollars.",
    },
    "after": {
        "caption": "The points show each day's range. Saturday has the widest spread.",
        "summary": "Saturday tips ranged from 1.00 to 10.00 dollars. Friday tips ranged "
        "from 1.00 to 4.73 dollars.",
    },
}
