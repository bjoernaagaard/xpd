import plotly.express as px
import plotly.graph_objects as go


def load_data():
    frame = px.data.gapminder()
    selected = frame[
        (frame["year"] == 2007)
        & frame["country"].isin(["Austria", "Belgium", "Denmark", "Finland", "France"])
    ]
    return sorted(
        selected[["country", "lifeExp"]].itertuples(index=False, name=None),
        key=lambda row: row[1],
        reverse=True,
    )


def value_table(data):
    return {"columns": ["Country", "Life expectancy (years)"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    countries, values = zip(*sorted(data), strict=True)
    fig = go.Figure(go.Bar(x=countries, y=values))
    fig.update_layout(template="simple_white", yaxis_title="Life expectancy (years)")
    return fig


def make_after(data) -> go.Figure:
    countries, values = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=countries, y=values))
    fig.update_layout(template="simple_white", yaxis_title="Life expectancy (years)")
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset, five European countries in 2007.",
    "question": "How do these five countries rank by life expectancy?",
    "book": {
        "quote": "to name things (like an identification number) and to reflect an ordering",
        "page": 150,
        "line": 5758,
    },
    "before": {
        "caption": "Alphabetical order separates nearby values and leaves the rank to the reader.",
        "summary": "Life expectancy in 2007 was about 80.7 years in France and 78.3 in Denmark.",
    },
    "after": {
        "caption": "The bars and table begin with the longest life expectancy. France ranks first.",
        "summary": "Life expectancy in 2007 was about 80.7 years in France and 78.3 in Denmark.",
    },
}
