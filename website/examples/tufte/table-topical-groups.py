import plotly.graph_objects as go


def load_data():
    return [
        ("Party", "Democrats (43%)", 66, 26, 6, "77 to 22"),
        ("Party", "Independents (23%)", 30, 54, 12, "43 to 54"),
        ("Party", "Republicans (28%)", 11, 84, 4, "9 to 90"),
        ("Ideology", "Liberals (17%)", 57, 27, 11, "70 to 26"),
        ("Ideology", "Moderates (46%)", 42, 48, 8, "51 to 48"),
        ("Ideology", "Conservatives (28%)", 23, 71, 4, "29 to 70"),
        ("Democrats", "Liberal Democrats (9%)", 70, 14, 13, "86 to 12"),
        ("Democrats", "Moderate Democrats (22%)", 66, 28, 6, "77 to 22"),
        ("Democrats", "Conservative Democrats (8%)", 53, 41, 4, "64 to 35"),
        ("Democrats", "Politically active Democrats (3%)", 72, 19, 8, None),
        ("Independents", "Liberal Independents (4%)", 50, 29, 15, "64 to 29"),
        ("Independents", "Moderate Independents (12%)", 31, 53, 13, "45 to 53"),
        ("Independents", "Conservative Independents (7%)", 22, 69, 6, "26 to 72"),
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
    interleaved = data[::3] + data[1::3] + data[2::3]
    values = list(zip(*(row[1:] for row in interleaved), strict=True))
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
            cells={"values": values, "align": ["left", "right", "right", "right", "right"]},
            columnwidth=[2, 1, 1, 1, 1],
        )
    )
    fig.update_layout(
        template="simple_white", height=460, margin={"l": 12, "r": 12, "t": 52, "b": 12}
    )
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    topics = ("Party", "Ideology", "Democrats", "Independents")
    align = ["left", "right", "right", "right", "right"]
    for index, topic in enumerate(topics):
        group = [
            tuple(value if value is not None else "Missing" for value in row[1:])
            for row in data
            if row[0] == topic
        ]
        top = 1 - 0.25 * index
        fig.add_trace(
            go.Table(
                domain={"y": [top - 0.23, top - 0.045]},
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
                    "height": 28,
                },
                cells={
                    "values": list(zip(*group, strict=True)),
                    "align": align,
                    "fill": {"color": "rgba(0,0,0,0)"},
                    "line": {"width": 0},
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
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        height=760,
        width=700,
        margin={"l": 0, "r": 0, "t": 12, "b": 8},
    )
    return fig


EXAMPLE = {
    "source": "Election day voter surveys reproduced in Tufte's book. The 1980 sample "
    "included 12,782 interviews. The percentage in parentheses gives each group's "
    "share of voters.",
    "question": "How did presidential votes differ by party and ideology in 1980 and 1976?",
    "book": {
        "quote": "the rows are ordered so as to tell an ordered story",
        "page": 179,
        "line": 7178,
    },
    "before": {
        "caption": "The flat table mixes party and ideology rows. Comparisons within a "
        "topic require searching through other rows.",
        "summary": "In 1980 Carter took 66 percent of Democratic votes and Reagan took "
        "84 percent of Republican votes.",
    },
    "after": {
        "caption": "Four named groups put related comparisons together. Carter led "
        "among Democrats, and Reagan led among Republicans.",
        "summary": "In 1980 Carter took 66 percent of Democratic votes and Reagan took "
        "84 percent of Republican votes.",
    },
}
