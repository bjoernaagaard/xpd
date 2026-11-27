import plotly.express as px
import plotly.graph_objects as go


def load_data():
    stocks = px.data.stocks().iloc[:53]
    return [
        (ticker, row.date, float(getattr(row, ticker)))
        for ticker in ("GOOG", "AAPL", "AMZN")
        for row in stocks.itertuples(index=False)
    ]


def value_table(data):
    return {
        "columns": ["Ticker", "Week", "Index (first week = 1)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    fig = go.Figure()
    tickers = ("GOOG", "AAPL", "AMZN")
    for index, ticker in enumerate(tickers):
        dates, values = zip(
            *((date, value) for name, date, value in data if name == ticker), strict=True
        )
        fig.add_trace(
            go.Scatter(
                x=dates,
                y=values,
                name=ticker,
                mode="lines",
                line={"width": 2, "dash": ("solid", "dash", "dot")[index]},
            )
        )
    fig.update_layout(template="simple_white", yaxis_title="Index (first week = 1)")
    fig.update_xaxes(showgrid=True, gridcolor="#bbbbbb")
    fig.update_yaxes(showgrid=True, gridcolor="#bbbbbb")
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    tickers = ("GOOG", "AAPL", "AMZN")
    for index, ticker in enumerate(tickers):
        dates, values = zip(
            *((date, value) for name, date, value in data if name == ticker), strict=True
        )
        fig.add_trace(
            go.Scatter(
                x=dates,
                y=values,
                name=ticker,
                mode="lines",
                line={"width": 2, "dash": ("solid", "dash", "dot")[index]},
            )
        )
    fig.update_layout(template="simple_white", yaxis_title="Index (first week = 1)")
    fig.add_hline(y=1, line_width=1, line_color="#888888", annotation_text="Starting value")
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Stocks dataset, first 53 weekly observations.",
    "question": "When are these stocks above their starting value?",
    "book": {
        "quote": "The grid ticks compete with the essential information",
        "page": 105,
        "line": 3973,
    },
    "before": {
        "caption": "The full grid gives the starting value the same weight as other ticks.",
        "summary": "All start at 1.00. During the week of March 19, Amazon is about "
        "1.22, Apple 0.94 and Google 0.93.",
    },
    "after": {
        "caption": "One reference at 1 shows when the indexes are above or below their "
        "starting values.",
        "summary": "All start at 1.00. During the week of March 19, Amazon is about "
        "1.22, Apple 0.94 and Google 0.93.",
    },
}
