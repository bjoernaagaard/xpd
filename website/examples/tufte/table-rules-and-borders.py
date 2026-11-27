import plotly.graph_objects as go


def load_data():
    return [
        ("Party", "Democrats (43%)", 66, 26, 6, "77 to 22"),
        ("Party", "Independents (23%)", 30, 54, 12, "43 to 54"),
        ("Party", "Republicans (28%)", 11, 84, 4, "9 to 90"),
        ("Ideology", "Liberals (17%)", 57, 27, 11, "70 to 26"),
        ("Ideology", "Moderates (46%)", 42, 48, 8, "51 to 48"),
        ("Ideology", "Conservatives (28%)", 23, 71, 4, "29 to 70"),
    ]


def value_table(data):
    return {
        "columns": [
            "Topic",
            "Voter group",
            "Carter 1980 (%)",
            "Reagan 1980 (%)",
            "Anderson 1980 (%)",
            "Carter and Ford 1976 (%)",
        ],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    values = list(zip(*(row[1:] for row in data), strict=True))
    fig = go.Figure(
        go.Table(
            header={
                "values": [
                    "Voter group",
                    "Carter 1980 (%)",
                    "Reagan 1980 (%)",
                    "Anderson 1980 (%)",
                    "Carter and Ford 1976 (%)",
                ],
                "align": ["left", "right", "right", "right", "right"],
            },
            cells={
                "values": values,
                "align": ["left", "right", "right", "right", "right"],
                "line": {"width": 3, "color": "#606060"},
                "fill": {"color": ["#ffffff", "#dbe7f0"]},
            },
            columnwidth=[2, 1, 1, 1, 1],
        )
    )
    fig.update_layout(
        template="simple_white", height=360, margin={"l": 12, "r": 12, "t": 52, "b": 12}
    )
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    for index, topic in enumerate(("Party", "Ideology")):
        group = [row[1:] for row in data if row[0] == topic]
        top = 1 - 0.5 * index
        fig.add_trace(
            go.Table(
                domain={"y": [top - 0.43, top - 0.05]},
                columnwidth=[2, 1, 1, 1, 1],
                header={
                    "values": [
                        "Voter group",
                        "Carter 1980 (%)",
                        "Reagan 1980 (%)",
                        "Anderson 1980 (%)",
                        "Carter and Ford 1976 (%)",
                    ],
                    "height": 32,
                },
                cells={
                    "values": list(zip(*group, strict=True)),
                    "line": {"width": 0},
                    "fill": {"color": "#ffffff"},
                    "align": ["left", "right", "right", "right", "right"],
                    "height": 26,
                },
            )
        )
        fig.add_annotation(
            x=0, y=top, xref="paper", yref="paper", text=topic, showarrow=False, yanchor="top"
        )
    fig.add_shape(
        type="line",
        x0=0,
        x1=1,
        y0=0.53,
        y1=0.53,
        xref="paper",
        yref="paper",
        line={"width": 1, "color": "#b0b0b0"},
    )
    fig.update_layout(
        template="simple_white", height=455, margin={"l": 12, "r": 12, "t": 20, "b": 10}
    )
    return fig


EXAMPLE = {
    "source": "Election day voter surveys reproduced in Tufte's book. The 1980 sample "
    "included 12,782 interviews. The percentage in parentheses gives each group's "
    "share of voters.",
    "question": "How did Carter's support vary across party and ideology?",
    "book": {
        "quote": "The horizontal rules divide the data into topical paragraphs",
        "page": 179,
        "line": 7178,
    },
    "before": {
        "caption": "Thick borders and alternating fills give every cell an outline or "
        "stripe, including those within the same topic.",
        "summary": "In 1980 Carter received 66 percent among Democrats, 30 percent "
        "among independents and 11 percent among Republicans.",
    },
    "after": {
        "caption": "A single light rule marks the topic boundary and leaves the rows easy to scan.",
        "summary": "In 1980 Carter received 66 percent among Democrats, 30 percent "
        "among independents and 11 percent among Republicans.",
    },
}
