import plotly.graph_objects as go


def load_data():
    return [
        ("Reported estimate", 3.8, 5.8),
        ("Conference Board", 4.2, 5.5),
        ("Data Resources", 4.5, 5.2),
        ("Wharton", 4.5, 4.8),
        ("Chase Econometrics", 2.8, 1.9),
    ]


def value_table(data):
    return {
        "columns": ["Forecaster", "Real GNP growth (%)", "Industrial production growth (%)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    fig = go.Figure(
        go.Table(
            header={"values": ["Forecaster", "Real GNP (%)", "Industrial production (%)"]},
            cells={
                "values": list(zip(*data, strict=True)),
                "align": ["left", "right", "right"],
                "format": ["", ".1f", ".1f"],
            },
            columnwidth=[2, 1, 1],
        )
    )
    fig.update_layout(
        template="simple_white", height=300, margin={"l": 15, "r": 15, "t": 42, "b": 12}
    )
    return fig


def make_after(data) -> go.Figure:
    from plotly.subplots import make_subplots

    names, gnp, industry = zip(*data, strict=True)
    fig = make_subplots(
        rows=1,
        cols=3,
        specs=[[{"type": "table"}, {"type": "xy"}, {"type": "xy"}]],
        subplot_titles=("", "Real GNP growth", "Industrial production"),
        column_widths=[0.52, 0.24, 0.24],
        horizontal_spacing=0.03,
    )
    fig.add_trace(
        go.Table(
            header={"values": ["Forecaster", "GNP (%)", "Industry (%)"], "height": 24},
            cells={
                "values": [names, gnp, industry],
                "align": ["left", "right", "right"],
                "format": ["", ".1f", ".1f"],
                "height": 34,
            },
            columnwidth=[2, 1, 1],
        ),
        row=1,
        col=1,
    )
    rows = [41 + index * 34 for index in range(len(names))]
    for column, values in ((2, gnp), (3, industry)):
        fig.add_trace(
            go.Scatter(
                x=values,
                y=rows,
                mode="markers",
                showlegend=False,
                marker={
                    "color": "#245a80",
                    "symbol": ["diamond", "circle", "circle", "circle", "circle"],
                },
                hovertext=names,
            ),
            row=1,
            col=column,
        )
        xref, yref = ("x", "y domain") if column == 2 else ("x2", "y2 domain")
        fig.add_shape(
            type="line",
            x0=values[0],
            x1=values[0],
            y0=0,
            y1=1,
            xref=xref,
            yref=yref,
            line={"color": "#a0a0a0", "width": 1},
        )
        fig.update_yaxes(range=[194, 0], visible=False, row=1, col=column)
        fig.update_xaxes(range=[0, 7], tickvals=[0, 3, 6], ticksuffix="%", row=1, col=column)
    fig.update_layout(
        template="simple_white", height=260, margin={"l": 10, "r": 12, "t": 30, "b": 36}
    )
    return fig


EXAMPLE = {
    "source": "1978 forecasts and probable results from the New York Times graphic reproduced "
    "in Tufte's book.",
    "question": "Which forecasts came closest to the reported 1978 estimates?",
    "book": {
        "quote": "For sets of highly labeled numbers",
        "page": 180,
        "line": 7755,
    },
    "before": {
        "caption": "The table gives the exact forecasts. To compare each one with the "
        "reported estimate, the reader has to subtract values.",
        "summary": "The reported estimates were 3.8 percent for real GNP growth and "
        "5.8 percent for industrial production. Conference Board forecast "
        "4.2 and 5.5 percent.",
    },
    "after": {
        "caption": "The dots beside the table show the differences from the reported "
        "estimates. Conference Board is nearest on both measures.",
        "summary": "The reported estimates were 3.8 percent for real GNP growth and 5.8 "
        "percent for industrial production. Conference Board forecast 4.2 "
        "and 5.5 percent.",
    },
}
