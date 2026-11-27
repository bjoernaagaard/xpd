import plotly.express as px
import plotly.graph_objects as go


def load_data():
    frame = px.data.gapminder()
    subset = frame[
        (frame["year"] == 2007)
        & frame["country"].isin(
            ["Bosnia and Herzegovina", "Czech Republic", "Slovak Republic", "United Kingdom"]
        )
    ]
    return list(subset[["country", "lifeExp"]].itertuples(index=False, name=None))


def value_table(data):
    return {"columns": ["Country", "Life expectancy (years)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    countries, values = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=values, y=countries, orientation="h"))
    fig.update_layout(
        template="simple_white",
        width=365,
        height=360,
        margin={"l": 26},
        yaxis_automargin=False,
        xaxis_title="Life expectancy (years)",
    )
    return fig


def make_after(data) -> go.Figure:
    countries, values = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=values, y=countries, orientation="h"))
    fig.update_layout(
        template="simple_white",
        width=560,
        height=340,
        yaxis_automargin=True,
        margin={"l": 151, "r": 12},
        xaxis_title="Life expectancy (years)",
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "Which of these countries had the highest life expectancy in 2007?",
    "book": {
        "quote": "with the technical details of production done with care",
        "page": 177,
        "line": 7115,
    },
    "before": {
        "caption": "The narrow left margin clips the country names.",
        "summary": "The United Kingdom reached 79.4 years and the Slovak Republic "
        "reached 74.7 years in 2007.",
    },
    "after": {
        "caption": "The chart reserves space for each name. The United Kingdom has the "
        "longest bar.",
        "summary": "The United Kingdom reached 79.4 years and the Slovak Republic "
        "reached 74.7 years in 2007.",
    },
}
