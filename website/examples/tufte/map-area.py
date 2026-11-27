from math import sqrt

import plotly.express as px
import plotly.graph_objects as go


def load_data():
    land_area = {"Australia": 7682300, "Japan": 364500, "Bangladesh": 130170}
    frame = px.data.gapminder()
    selected = frame[(frame["year"] == 2007) & frame["country"].isin(land_area)]
    return sorted(
        [
            (country, int(population), land_area[country])
            for country, population in selected[["country", "pop"]].itertuples(
                index=False, name=None
            )
        ],
        key=lambda row: row[1],
    )


def value_table(data):
    return {
        "columns": ["Country", "Population in 2007", "Land area (km²)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    positions = {"Australia": (6, 2), "Japan": (7, 5), "Bangladesh": (3, 4)}
    fig = go.Figure()
    for country, population, area in data:
        x, y = positions[country]
        side = sqrt(area / 1100000)
        corner = side / 2
        fig.add_trace(
            go.Scatter(
                x=[x - corner, x + corner, x + corner, x - corner, x - corner],
                y=[y - corner, y - corner, y + corner, y + corner, y - corner],
                name=country,
                mode="lines",
                fill="toself",
                fillcolor=f"rgba(36, 90, 128, {0.2 + population / 220000000})",
                line={"color": "#245a80"},
                hovertemplate=f"{country}: {population:,} people<extra></extra>",
            )
        )
        fig.add_annotation(x=x, y=y + corner, text=country, showarrow=False, yshift=12)
    fig.update_layout(
        template="simple_white",
        showlegend=False,
        xaxis={"visible": False, "range": [1, 9]},
        yaxis={"visible": False, "range": [0, 6], "scaleanchor": "x"},
        title="Population by region",
    )
    return fig


def make_after(data) -> go.Figure:
    countries, population, _ = zip(*data, strict=True)
    fig = go.Figure(
        go.Bar(x=[value / 1000000 for value in population], y=countries, orientation="h")
    )
    fig.update_layout(
        template="simple_white", xaxis_title="Population (millions)", yaxis_automargin=True
    )
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Population from Plotly Express Gapminder. Rounded land areas from the World "
    "Bank land area indicator.",
    "question": "Which country had the largest population in 2007?",
    "book": {
        "quote": "visual importance of each county with its geographic area",
        "page": 24,
        "line": 427,
    },
    "before": {
        "caption": "The schematic places land area in the size of each region and "
        "population in its shade. Australia occupies most of the drawing "
        "even though it has the smallest population.",
        "summary": "Bangladesh had about 150 million people, Japan 127 million and "
        "Australia 20 million.",
    },
    "after": {
        "caption": "Bars of equal thickness use length for population. Bangladesh has "
        "the longest bar.",
        "summary": "Bangladesh had about 150 million people, Japan 127 million and "
        "Australia 20 million.",
    },
}
