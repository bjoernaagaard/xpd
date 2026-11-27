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
    fig = go.Figure(
        go.Bar(
            x=values,
            y=countries,
            orientation="h",
            text=["2007"] * len(values),
            textposition="inside",
        )
    )
    fig.update_layout(
        template="simple_white", yaxis_automargin=True, xaxis_title="Life expectancy (years)"
    )
    return fig


def make_after(data) -> go.Figure:
    countries, values = zip(*data, strict=True)
    fig = go.Figure(go.Bar(x=values, y=countries, orientation="h"))
    fig.update_layout(
        template="simple_white", yaxis_automargin=True, xaxis_title="Life expectancy (years)"
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "How did life expectancy vary among these countries in 2007?",
    "book": {"quote": "while still retaining all the data", "page": 103, "line": 3949},
    "before": {
        "caption": "Each bar repeats the year. The repeated label has no bearing on "
        "the differences between countries.",
        "summary": "The United Kingdom reached 79.4 years in 2007 and the Czech "
        "Republic reached 76.5.",
    },
    "after": {
        "caption": "All bars show 2007 life expectancy. The caption states the year "
        "once and the bars carry the comparisons.",
        "summary": "The United Kingdom reached 79.4 years in 2007 and the Czech "
        "Republic reached 76.5.",
    },
}
