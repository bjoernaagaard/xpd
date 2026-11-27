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
        color_discrete_sequence=["#3b82c4", "#d97706", "#8b5cf6", "#14967f", "#78716c"],
    )
    fig.update_traces(
        marker={"size": 7, "opacity": 0.9},
        hovertemplate="<b>%{hovertext}</b><br>Income: $%{x:,.0f}<br>Life expectancy: %{y:.1f} yr<extra>%{fullData.name}</extra>",
    )
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        title={
            "text": "Income and life expectancy by continent, 2007",
            "x": 0,
            "xanchor": "left",
            "subtitle": {"text": "GDP per capita on a log scale versus life expectancy at birth"},
        },
        xaxis_tickprefix="$",
        yaxis_title="Life expectancy (years)",
        legend={"orientation": "h", "y": 1.06, "x": 0, "xanchor": "left", "title": {"text": ""}},
        margin={"l": 64, "r": 24, "t": 100, "b": 64},
    )
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
