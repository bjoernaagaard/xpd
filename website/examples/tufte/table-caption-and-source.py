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
                "values": ["Voter group", "Carter", "Reagan", "Anderson", "Carter and Ford"],
                "align": ["left", "right", "right", "right", "right"],
            },
            cells={"values": values, "align": ["left", "right", "right", "right", "right"]},
            columnwidth=[2, 1, 1, 1, 1],
        )
    )
    fig.update_layout(
        template="simple_white", height=360, margin={"l": 12, "r": 12, "t": 52, "b": 12}
    )
    return fig


def make_after(data) -> go.Figure:
    values = list(zip(*(row[1:] for row in data), strict=True))
    align = ["left", "right", "right", "right", "right"]
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
                "align": align,
                "fill": {"color": "rgba(15,23,42,0.04)"},
                "line": {"color": "#cbd5e1", "width": 1},
                "font": {"color": "#1f2937", "size": 12},
                "height": 34,
            },
            cells={
                "values": values,
                "align": align,
                "fill": {"color": "rgba(0,0,0,0)"},
                "line": {"width": 0},
                "font": {"color": "#1f2937", "size": 12},
                "height": 28,
            },
            columnwidth=[2, 1, 1, 1, 1],
        )
    )
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        height=290,
        margin={"l": 0, "r": 0, "t": 64, "b": 8},
        title={
            "text": "Presidential votes by group, 1980 and 1976",
            "x": 0,
            "xanchor": "left",
            "subtitle": {"text": "Share of voters (%) from 12,782 election-day interviews"},
        },
    )
    return fig


EXAMPLE = {
    "source": "Election day voter surveys reproduced in Tufte's book. The 1980 sample "
    "included 12,782 interviews. The percentage in parentheses gives each group's "
    "share of voters.",
    "question": "What do the 1980 and 1976 presidential vote figures represent?",
    "book": {"quote": "Based on 12,782 interviews with voters", "page": 179, "line": 7182},
    "before": {
        "caption": "The table provides candidate names but no year, unit or survey population.",
        "summary": "Among Democrats interviewed in 1980, 66 percent voted for Carter. "
        "The 1976 comparison was 77 percent for Carter and 22 percent for "
        "Ford.",
    },
    "after": {
        "caption": "The title, subtitle, and headers name the elections, units, and sample size.",
        "summary": "Among Democrats interviewed in 1980, 66 percent voted for Carter. "
        "The 1976 comparison was 77 percent for Carter and 22 percent for "
        "Ford.",
    },
}
