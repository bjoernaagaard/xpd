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
    "question": "Which line ends higher, Apple or Google?",
    "book": {"quote": "labels are placed on the graphic itself", "page": 183, "line": 7978},
    "before": {
        "caption": "The legend supplies the names while the ends of the lines show their values.",
        "summary": "During the week of March 19, Apple is about 0.94 and Google 0.93. "
        "Both started at 1.00.",
    },
    "after": {
        "caption": "The names at the line ends let readers identify the lines at the "
        "point of comparison.",
        "summary": "During the week of March 19, Apple is about 0.94 and Google 0.93. "
        "Both started at 1.00.",
    },
}
