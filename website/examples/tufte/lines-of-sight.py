import plotly.express as px
import plotly.graph_objects as go


def load_data():
    stocks = px.data.stocks().iloc[:53]
    return [
        (ticker, row.date, float(getattr(row, ticker)))
        for index, ticker in enumerate(("GOOG", "AAPL", "AMZN"))
        for row in stocks.iloc[index * 8 :].itertuples(index=False)
    ]


def value_table(data):
    return {
        "columns": ["Ticker", "Week", "Index (first week = 1)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(data, columns=["Ticker", "Week", "Index"])
    fig = px.line(
        frame,
        x="Week",
        y="Index",
        facet_row="Ticker",
        template="simple_white",
        height=540,
        width=700,
    )
    fig.update_xaxes(matches=None)
    fig.for_each_annotation(lambda note: note.update(text=note.text.split("=")[-1]))
    return fig


def make_after(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(data, columns=["Ticker", "Week", "Index"])
    fig = px.line(
        frame,
        x="Week",
        y="Index",
        facet_row="Ticker",
        template="simple_white",
        height=540,
        width=700,
    )
    fig.update_xaxes(matches="x", nticks=4)
    fig.for_each_annotation(lambda note: note.update(text=note.text.split("=")[-1]))
    return fig


EXAMPLE = {
    "source": "Plotly Express Stocks dataset, with different starting dates to show axis "
    "alignment.",
    "question": "How can you compare the stocks during the same weeks?",
    "book": {
        "quote": "Each separate line of sight should remain unchanging",
        "page": 156,
        "line": 5960,
    },
    "before": {
        "caption": "Each panel starts its scale at that stock's first visible "
        "observation. The same week lands in different horizontal "
        "positions.",
        "summary": "At the end of 2018, Amazon was about 1.28, Google 0.97 and Apple 0.85.",
    },
    "after": {
        "caption": "All three panels share the date scale. Reading vertically compares "
        "indexes for the same week.",
        "summary": "At the end of 2018, Amazon was about 1.28, Google 0.97 and Apple 0.85.",
    },
}
