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
                line={"width": 6, "dash": ("solid", "dash", "dot")[index]},
            )
        )
    fig.update_layout(template="simple_white", yaxis_title="Index (first week = 1)")
    fig.add_hline(y=1, line_width=6, line_color="#444444")
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
                line={"width": 1.5, "dash": ("solid", "dash", "dot")[index]},
            )
        )
    fig.update_layout(template="simple_white", yaxis_title="Index (first week = 1)")
    fig.add_hline(y=1, line_width=1, line_color="#a1a1a1")
    return fig


EXAMPLE = {
    "source": "Plotly Express Stocks dataset, first 53 weekly observations.",
    "question": "Which stocks finish above their starting value?",
    "book": {"quote": "Lines in data graphics should be thin", "page": 185, "line": 8068},
    "before": {
        "caption": "Thick lines and a heavy reference compete where the series cross.",
        "summary": "At the start all three equal 1.00. During the week of March 19, "
        "Amazon is about 1.22, Apple 0.94 and Google 0.93.",
    },
    "after": {
        "caption": "Thin lines make the crossings visible. A light line marks the "
        "starting index of 1.",
        "summary": "At the start all three equal 1.00. During the week of March 19, "
        "Amazon is about 1.22, Apple 0.94 and Google 0.93.",
    },
}
