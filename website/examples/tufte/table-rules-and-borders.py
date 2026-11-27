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
    fig.update_layout(template="simple_white", height=320, margin={"l": 0, "r": 0, "t": 16, "b": 8})
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    align = ["left", "right", "right", "right", "right"]
    for index, topic in enumerate(("Party", "Ideology")):
        group = [row[1:] for row in data if row[0] == topic]
        top = 1 - 0.5 * index
        fig.add_trace(
            go.Table(
                domain={"y": [top - 0.43, top - 0.09]},
                columnwidth=[2, 1, 1, 1, 1],
                header={
                    "values": [
                        "Voter group",
                        "Carter 1980 (%)",
                        "Reagan 1980 (%)",
                        "Anderson 1980 (%)",
                        "Carter and Ford 1976 (%)",
                    ],
                    "align": align,
                    "fill": {"color": "rgba(15,23,42,0.04)"},
                    "line": {"color": "#cbd5e1", "width": 1},
                    "font": {"color": "#1f2937", "size": 12},
                    "height": 32,
                },
                cells={
                    "values": list(zip(*group, strict=True)),
                    "line": {"width": 0},
                    "fill": {"color": "rgba(0,0,0,0)"},
                    "align": align,
                    "font": {"color": "#1f2937", "size": 12},
                    "height": 26,
                },
            )
        )
        fig.add_annotation(
            x=0,
            y=top,
            xref="paper",
            yref="paper",
            text=f"<b>{topic}</b>",
            showarrow=False,
            xanchor="left",
            yanchor="top",
            font={"size": 13},
        )
    fig.add_shape(
        type="line",
        x0=0,
        x1=1,
        y0=0.53,
        y1=0.53,
        xref="paper",
        yref="paper",
        line={"width": 1, "color": "#cbd5e1"},
    )
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        height=390,
        margin={"l": 0, "r": 0, "t": 12, "b": 8},
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
