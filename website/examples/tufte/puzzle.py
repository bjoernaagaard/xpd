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
    import pandas as pd
    import plotly.express as px

    frame = pd.DataFrame(data, columns=["Ticker", "Week", "Index"])
    return px.line(
        frame,
        x="Week",
        y="Index",
        color="Ticker",
        line_dash="Ticker",
        template="simple_white",
        labels={"Index": "Index (first week = 1)"},
    )


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
        fig.add_annotation(
            x=dates[-1], y=values[-1], text=ticker, showarrow=False, xanchor="left", xshift=4
        )
    fig.update_layout(
        template="simple_white",
        showlegend=False,
        yaxis_title="Index (first week = 1)",
        margin={"r": 64},
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Stocks dataset, first 53 weekly observations.",
    "question": "Which stock index is highest at the end of 2018?",
    "book": {"quote": "The danger of multifunctioning elements", "page": 140, "line": 5260},
    "before": {
        "caption": "A separate key assigns names to line styles. The reader must look "
        "away from the endpoints to identify them.",
        "summary": "All three indexes start at 1.00. Amazon finishes 2018 above Apple and Google.",
    },
    "after": {
        "caption": "Names at the endpoints identify each index where the comparison is made.",
        "summary": "All three indexes start at 1.00. Amazon finishes 2018 above Apple and Google.",
    },
}
