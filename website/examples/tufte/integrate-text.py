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
    fig.add_annotation(
        x=0.5,
        y=1.06,
        xref="paper",
        yref="paper",
        text="See the caption for the comparison",
        showarrow=False,
    )
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
    fig.update_layout(font_family="system-ui", margin={"r": 120})
    fig.add_annotation(x=2007, y=78.6, text="4 years apart", showarrow=True, ax=-30, ay=35)
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "When did the life expectancy gap narrow?",
    "book": {
        "quote": "be closely integrated with the statistical and verbal descriptions",
        "page": 17,
        "line": 116,
    },
    "before": {
        "caption": "The text above the figure sends the reader to the caption to find "
        "the comparison.",
        "summary": "In 1952 the gap was about 15.5 years. In 2007 it was about 4.0 years.",
    },
    "after": {
        "caption": "A note at the 2007 endpoint shows where the difference was about four years.",
        "summary": "In 1952 the gap was about 15.5 years. In 2007 it was about 4.0 years.",
    },
}
