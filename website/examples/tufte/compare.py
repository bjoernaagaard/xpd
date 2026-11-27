import pandas as pd
import plotly.express as px
import plotly.graph_objects as go


def load_data():
    frame = px.data.gapminder()
    selected = frame[frame["country"].isin(["Japan", "Korea, Rep."])]
    return [
        ("South Korea" if country == "Korea, Rep." else country, year, life)
        for country, year, life in selected[["country", "year", "lifeExp"]].itertuples(
            index=False, name=None
        )
    ]


def value_table(data):
    return {
        "columns": ["Country", "Year", "Life expectancy (years)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    frame = pd.DataFrame(data, columns=["Country", "Year", "Life expectancy (years)"])
    fig = px.line(
        frame, x="Year", y="Life expectancy (years)", facet_col="Country", template="simple_white"
    )
    fig.update_yaxes(matches=None)
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    for name in ("Japan", "South Korea"):
        years, values = zip(
            *((year, life) for country, year, life in data if country == name), strict=True
        )
        fig.add_trace(
            go.Scatter(
                x=years,
                y=values,
                mode="lines",
                name=name,
                line={"dash": "solid" if name == "Japan" else "dash"},
            )
        )
    fig.update_layout(
        template="simple_white", xaxis_title="Year", yaxis_title="Life expectancy (years)"
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "How did life expectancy in Japan and South Korea change?",
    "book": {"quote": "encourage the eye to compare", "page": 17, "line": 113},
    "before": {
        "caption": "Separate panels put each country's values on its own scale. The "
        "starting gap is hard to see.",
        "summary": "Life expectancy in 1952 was 63.0 years in Japan and 47.5 in South "
        "Korea. In 2007 it was 82.6 and 78.6 years.",
    },
    "after": {
        "caption": "Two lines on one scale show South Korea approaching Japan's life expectancy.",
        "summary": "Life expectancy in 1952 was 63.0 years in Japan and 47.5 in South "
        "Korea. In 2007 it was 82.6 and 78.6 years.",
    },
}
