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
                line={"width": 2, "color": ("#bb0000", "#008000")[index]},
            )
        )
    fig.update_layout(template="simple_white", yaxis_title="Index (first week = 1)")
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
                line={
                    "width": 2,
                    "color": ("#245a80", "#a35a00")[index],
                    "dash": ("solid", "dash")[index],
                },
            )
        )
        fig.add_annotation(
            x=dates[-1], y=values[-1], text=ticker, showarrow=False, xanchor="left", xshift=4
        )
    fig.update_layout(
        template="simple_white",
        showlegend=False,
        yaxis_title="Index (first week = 1)",
        margin={"r": 64},
        yaxis_range=[0.74, 1.36],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Stocks dataset, first 53 weekly observations.",
    "question": "How can you follow Apple and Google where the lines are close?",
    "book": {
        "quote": "colors, if used, are chosen so that the color-deficient",
        "page": 183,
        "line": 7986,
    },
    "before": {
        "caption": "The original chart uses solid red and green lines for the two indexes. "
        "A reader who cannot distinguish those colors can lose track of the "
        "lines. The chart on this page uses dashes for Apple. The downloaded before "
        "code uses two solid lines.",
        "summary": "On December 31, Google was about 0.97 and Apple 0.85. Both started at 1.00.",
    },
    "after": {
        "caption": "The solid and dashed lines remain distinct without color, and the "
        "line names appear at the ends.",
        "summary": "On December 31, Google was about 0.97 and Apple 0.85. Both started at 1.00.",
    },
}
