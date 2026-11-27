import plotly.express as px
import plotly.graph_objects as go


def load_data():
    frame = px.data.gapminder()
    return list(
        frame[["country", "year", "gdpPercap", "lifeExp", "continent"]].itertuples(
            index=False, name=None
        )
    )


def value_table(data):
    return {
        "columns": ["Country", "Year", "Income per person", "Life expectancy (years)", "Continent"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(
        data, columns=["Country", "Year", "Income per person", "Life expectancy", "Continent"]
    )
    current = frame[frame["Year"] == 2007]
    fig = px.scatter(
        current,
        x="Income per person",
        y="Life expectancy",
        log_x=True,
        template="simple_white",
        hover_name="Country",
    )
    return fig


def make_after(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(
        data, columns=["Country", "Year", "Income per person", "Life expectancy", "Continent"]
    )
    current = frame[frame["Year"] == 2007]
    fig = px.scatter(
        current,
        x="Income per person",
        y="Life expectancy",
        color="Continent",
        symbol="Continent",
        log_x=True,
        template="simple_white",
        hover_name="Country",
        color_discrete_sequence=["#245a80", "#8f591f", "#643e90", "#447d80", "#353535"],
    )
    fig.update_layout(legend={"orientation": "h", "y": 1.05})
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "Does the relationship between income and life expectancy differ by continent?",
    "book": {
        "quote": "Graphical excellence is nearly always multivariate",
        "page": 55,
        "line": 1354,
    },
    "before": {
        "caption": "The scatter shows the income and lifespan relationship. It gives "
        "no way to identify regional patterns.",
        "summary": "In 2007 life expectancy was 82.6 years in Japan and 64.1 in "
        "Bangladesh. Both are in Asia.",
    },
    "after": {
        "caption": "Colors and marker shapes identify continents. Many African "
        "countries appear in the lower left.",
        "summary": "In 2007 life expectancy was 82.6 years in Japan and 64.1 in "
        "Bangladesh. Both are in Asia.",
    },
}
