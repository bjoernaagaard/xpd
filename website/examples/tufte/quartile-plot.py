import plotly.express as px
import plotly.graph_objects as go


def load_data():
    tips = px.data.tips()
    return [
        (str(day), float(bill))
        for day, bill in tips[["day", "total_bill"]].itertuples(index=False, name=None)
    ]


def value_table(data):
    return {"columns": ["Day", "Bill (USD)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    fig = go.Figure()
    for day in ("Thur", "Fri", "Sat", "Sun"):
        bills = [bill for name, bill in data if name == day]
        fig.add_trace(go.Box(y=bills, name=day, showlegend=False))
    fig.update_traces(boxpoints=False)
    fig.update_layout(template="simple_white", yaxis_title="Bill (USD)")
    return fig


def make_after(data) -> go.Figure:
    from statistics import quantiles

    fig = go.Figure()
    for day in ("Thur", "Fri", "Sat", "Sun"):
        bills = sorted(bill for name, bill in data if name == day)
        lower, middle, upper = quantiles(bills, n=4, method="inclusive")
        iqr = upper - lower
        whiskers = [bill for bill in bills if lower - 1.5 * iqr <= bill <= upper + 1.5 * iqr]
        low, high = min(whiskers), max(whiskers)
        fig.add_trace(
            go.Scatter(
                x=[day, day],
                y=[low, high],
                mode="lines",
                line={"color": "#5c6980", "width": 1},
                showlegend=False,
                hoverinfo="skip",
            )
        )
        fig.add_trace(
            go.Scatter(
                x=[day] * 5,
                y=[low, lower, middle, upper, high],
                mode="markers",
                marker={
                    "color": "#245a80",
                    "symbol": ["line-ew", "line-ew", "circle", "line-ew", "line-ew"],
                    "line": {"width": 1.5},
                    "size": 12,
                },
                showlegend=False,
            )
        )
    fig.update_layout(template="simple_white", yaxis_title="Bill (USD)")
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Tips dataset.",
    "question": "How do the medians and spreads of bills compare by day?",
    "book": {"quote": "shows the same five numbers", "page": 126, "line": 4877},
    "before": {
        "caption": "An outline and colored fill surround the middle half of the "
        "bills for each day.",
        "summary": "The Friday median was 15.38 dollars. The Sunday median was 19.63 dollars.",
    },
    "after": {
        "caption": "Ticks and a dot mark quartiles and medians along the whisker ranges. "
        "Sunday has the highest median.",
        "summary": "The Friday median was 15.38 dollars. The Sunday median was 19.63 dollars.",
    },
}
