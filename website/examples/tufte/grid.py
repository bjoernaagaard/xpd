import plotly.express as px
import plotly.graph_objects as go


def load_data():
    stocks = px.data.stocks().iloc[:53]
    return [
        (ticker, row.date, float(getattr(row, ticker)))
        for ticker in ("GOOG", "AAPL")
        for row in stocks.itertuples(index=False)
    ]


def value_table(data):
    return {
        "columns": ["Ticker", "Week", "Index (first week = 1)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    fig = go.Figure()
    tickers = ("GOOG", "AAPL")
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
    fig.update_xaxes(showgrid=True, gridcolor="#000000", gridwidth=4)
    fig.update_yaxes(showgrid=True, gridcolor="#000000", gridwidth=4)
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    tickers = ("GOOG", "AAPL")
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
    fig.update_xaxes(showgrid=False)
    fig.update_yaxes(showgrid=True, gridcolor="#eeeeee", gridwidth=1)
    return fig


EXAMPLE = {
    "source": "Plotly Express Stocks dataset, first 53 weekly observations.",
    "question": "How did Apple and Google move over the year?",
    "book": {"quote": "the grid should usually be muted", "page": 114, "line": 4531},
    "before": {
        "caption": "The dark grid is thicker than the data lines. It competes with the "
        "two stock indexes.",
        "summary": "During the week of March 19, Apple is about 0.94 and Google 0.93. "
        "Both started at 1.00.",
    },
    "after": {
        "caption": "A pale value grid supports estimates and leaves both lines visible.",
        "summary": "During the week of March 19, Apple is about 0.94 and Google 0.93. "
        "Both started at 1.00.",
    },
}
