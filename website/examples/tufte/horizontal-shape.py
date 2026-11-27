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
    fig.update_layout(width=360, height=680)
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
    fig.update_layout(width=630, height=420)
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "How did the gap in life expectancy change over time?",
    "book": {"quote": "Graphics should tend toward the horizontal", "page": 186, "line": 8090},
    "before": {
        "caption": "A tall canvas leaves little horizontal room for five decades of observations.",
        "summary": "The gap narrowed from 15.5 years in 1952 to about four years in 2007.",
    },
    "after": {
        "caption": "The wider chart gives the dates more room and shows South Korea "
        "approaching Japan.",
        "summary": "The gap narrowed from 15.5 years in 1952 to about four years in 2007.",
    },
}
