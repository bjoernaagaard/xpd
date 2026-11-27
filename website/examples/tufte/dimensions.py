import plotly.graph_objects as go


def load_data():
    exports = {
        "Indochina": (1969, 2352, 2248),
        "Morocco": (1934, 2585, 2829),
        "Tunisia": (1243, 1894, 2832),
        "Algeria": (6646, 8217, 11610),
    }
    return [
        (territory, year, count)
        for territory, counts in exports.items()
        for year, count in zip(range(1927, 1930), counts, strict=True)
    ]


def value_table(data):
    return {"columns": ["Territory", "Year", "Car exports"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    fig = go.Figure()
    for territory in dict.fromkeys(row[0] for row in data):
        years, counts = zip(
            *((year, count) for name, year, count in data if name == territory), strict=True
        )
        fig.add_trace(
            go.Scatter3d(x=years, y=counts, z=[0] * len(years), name=territory, mode="markers")
        )
    fig.update_layout(
        template="simple_white",
        scene={"xaxis_title": "Year", "yaxis_title": "Cars", "zaxis_title": "Depth"},
    )
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    colors = ("#245a80", "#936022", "#774b8d", "#4c787b")
    for territory, color in zip(dict.fromkeys(row[0] for row in data), colors, strict=True):
        years, counts = zip(
            *((year, count) for name, year, count in data if name == territory), strict=True
        )
        fig.add_trace(go.Bar(x=years, y=counts, name=territory, marker_color=color))
    fig.update_layout(
        template="simple_white",
        barmode="group",
        yaxis_title="Car exports",
        xaxis_tickvals=[1927, 1928, 1929],
        legend={"orientation": "h", "y": 1.12},
    )
    return fig


EXAMPLE = {
    "source": "Car export counts from R. Satet's chart reproduced in Tufte's book.",
    "question": "How do car exports compare across territories and years?",
    "book": {
        "quote": "The number of information-carrying (variable) dimensions",
        "page": 75,
        "line": 2816,
    },
    "before": {
        "caption": "Perspective adds a depth axis even though all marks have the same depth.",
        "summary": "Algeria went from 6,646 cars in 1927 to 11,610 in 1929. Tunisia "
        "went from 1,243 to 2,832.",
    },
    "after": {
        "caption": "Grouped bars compare exports by year on a common scale. Algeria "
        "leads in each year.",
        "summary": "Algeria went from 6,646 cars in 1927 to 11,610 in 1929. Tunisia "
        "went from 1,243 to 2,832.",
    },
}
