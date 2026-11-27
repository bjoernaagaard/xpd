import plotly.graph_objects as go


def load_data():
    return [
        ("1973-01-01", 2.0),
        ("1974-01-01", 9.5),
        ("1975-01-01", 10.0),
        ("1976-01-01", 11.2),
        ("1977-01-01", 12.0),
        ("1978-01-01", 12.7),
        ("1979-01-01", 13.34),
        ("1979-04-01", 13.84),
        ("1979-07-01", 14.16),
        ("1979-10-01", 14.54),
    ]


def value_table(data):
    return {"columns": ["Date", "Oil price (USD per barrel)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    yearly, quarterly = data[:6], data[6:]
    fig = go.Figure()
    for rows, name, axis in ((yearly, "Annual price", "y"), (quarterly, "Quarterly price", "y2")):
        dates, prices = zip(*rows, strict=True)
        fig.add_trace(go.Scatter(x=dates, y=prices, name=name, yaxis=axis, mode="lines+markers"))
    fig.update_layout(
        template="simple_white",
        xaxis_type="category",
        xaxis_tickangle=-45,
        yaxis={"title": "USD per barrel", "range": [0, 15]},
        yaxis2={"title": "USD per barrel", "overlaying": "y", "side": "right", "range": [13, 14.7]},
    )
    return fig


def make_after(data) -> go.Figure:
    dates, prices = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=dates, y=prices, mode="lines+markers"))
    fig.update_layout(
        template="simple_white",
        xaxis_type="date",
        xaxis_title="Date",
        yaxis_title="Oil price (nominal USD per barrel)",
    )
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "OPEC prices in the New York Times chart reproduced by Tufte. Annual marks are "
    "approximate readings.",
    "question": "When did oil prices change most in this series?",
    "book": {"quote": "Show data variation", "page": 65, "line": 1863},
    "before": {
        "caption": "The quarterly prices use a separate vertical scale. Equal spacing "
        "also gives a quarter the same horizontal distance as a year.",
        "summary": "The annual price rose from about 2 dollars per barrel in 1973 to "
        "9.50 in 1974. In 1979 the price rose from 13.34 to 14.54 dollars "
        "per barrel.",
    },
    "after": {
        "caption": "A single value scale and actual date spacing show that the steepest "
        "increase occurred early in the series.",
        "summary": "The annual price rose from about 2 dollars per barrel in 1973 to "
        "9.50 in 1974. In 1979 the price rose from 13.34 to 14.54 dollars "
        "per barrel.",
    },
}
