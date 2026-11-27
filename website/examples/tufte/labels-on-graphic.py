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
    series = (("GOOG", "#3b82c4", "solid"), ("AAPL", "#d97706", "dash"))
    for ticker, color, dash in series:
        dates, values = zip(
            *((date, value) for name, date, value in data if name == ticker), strict=True
        )
        fig.add_trace(
            go.Scatter(
                x=dates,
                y=values,
                name=ticker,
                mode="lines",
                line={"width": 2, "color": color, "dash": dash},
                hovertemplate="%{y:.2f}<extra>%{fullData.name}</extra>",
            )
        )
        fig.add_annotation(
            x=dates[-1],
            y=values[-1],
            text=f"<b>{ticker}</b>  {values[-1]:.2f}",
            showarrow=False,
            xanchor="left",
            xshift=6,
            font={"color": color, "size": 12},
        )
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        title={
            "text": "Weekly stock index, 2018",
            "x": 0,
            "xanchor": "left",
            "subtitle": {"text": "Rebased to 1.00 on January 1, 2018"},
        },
        showlegend=False,
        hovermode="x unified",
        xaxis_title="Week",
        yaxis_title="Index (first week = 1)",
        yaxis_range=[0.76, 1.36],
        yaxis_tickformat=".2f",
        margin={"l": 64, "r": 96, "t": 84, "b": 64},
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
        "caption": "The names and final values sit cleanly beside the line ends with padded "
        "margins so readers identify each series at the point of comparison.",
        "summary": "During the week of March 19, Apple is about 0.94 and Google 0.93. "
        "Both started at 1.00.",
    },
}
