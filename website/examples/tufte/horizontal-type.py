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
    fig = go.Figure(go.Bar(x=countries, y=values))
    fig.update_layout(
        template="simple_white",
        xaxis_tickangle=-60,
        xaxis_automargin=True,
        yaxis_title="Life expectancy (years)",
    )
    return fig


def make_after(data) -> go.Figure:
    countries, values = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=values, y=countries, orientation="h"))
    fig.update_layout(
        template="simple_white",
        yaxis_automargin=True,
        margin={"l": 151, "r": 12},
        xaxis_title="Life expectancy (years)",
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "Which of these countries had the highest life expectancy in 2007?",
    "book": {"quote": "words run from left to right", "page": 183, "line": 7970},
    "before": {
        "caption": "The upright bars leave little room across the bottom. The country "
        "names run diagonally.",
        "summary": "The United Kingdom reached 79.4 years and Bosnia and Herzegovina "
        "reached 74.9 years in 2007.",
    },
    "after": {
        "caption": "Horizontal bars make room for country names that read from left to "
        "right. The United Kingdom has the longest bar.",
        "summary": "The United Kingdom reached 79.4 years and Bosnia and Herzegovina "
        "reached 74.9 years in 2007.",
    },
}
