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
    current = frame[frame["Year"] == 2007].rename(
        columns={"Income per person": "gdpPercap", "Life expectancy": "lifeExp"}
    )
    fig = px.scatter(current, x="gdpPercap", y="lifeExp", template="simple_white", log_x=True)
    return fig


def make_after(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(
        data, columns=["Country", "Year", "Income per person", "Life expectancy", "Continent"]
    )
    current = frame[frame["Year"] == 2007].rename(
        columns={"Income per person": "gdpPercap", "Life expectancy": "lifeExp"}
    )
    fig = px.scatter(
        current,
        x="gdpPercap",
        y="lifeExp",
        template="simple_white",
        log_x=True,
        labels={
            "gdpPercap": "Income per person (GDP per capita)",
            "lifeExp": "Life expectancy (years)",
        },
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "How is income per person related to life expectancy?",
    "book": {"quote": "words are spelled out", "page": 183, "line": 7966},
    "before": {
        "caption": "The axes use the names from the data file. The measures are hard "
        "to identify at a glance.",
        "summary": "In 2007 Japan had 82.6 years of life expectancy and about 31,656 "
        "in income per person. Bangladesh had 64.1 years and about 1,391.",
    },
    "after": {
        "caption": "The axes spell out the measures. Life expectancy is generally "
        "higher for countries with higher income per person.",
        "summary": "In 2007 Japan had 82.6 years of life expectancy and about 31,656 in "
        "income per person. Bangladesh had 64.1 years and about 1,391.",
    },
}
