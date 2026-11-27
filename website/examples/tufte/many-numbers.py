import plotly.express as px
import plotly.graph_objects as go


def load_data():
    frame = px.data.gapminder()
    selected = frame[frame["continent"].isin(["Asia", "Europe"])]
    return list(
        selected[["continent", "country", "year", "lifeExp"]].itertuples(index=False, name=None)
    )


def value_table(data):
    return {
        "columns": ["Continent", "Country", "Year", "Life expectancy (years)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(data, columns=["Continent", "Country", "Year", "Life expectancy (years)"])
    means = frame[frame["Year"] == 2007].groupby("Continent")["Life expectancy (years)"].mean()
    fig = go.Figure()
    for index, (region, mean) in enumerate(means.items()):
        fig.add_trace(
            go.Indicator(
                mode="gauge+number",
                value=mean,
                number={"valueformat": ".1f"},
                title={"text": region},
                gauge={"axis": {"range": [0, 90]}},
                domain={"row": 0, "column": index},
            )
        )
    fig.update_layout(
        template="simple_white", grid={"rows": 1, "columns": 2}, width=640, height=420
    )
    return fig


def make_after(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(data, columns=["Continent", "Country", "Year", "Life expectancy (years)"])
    fig = px.line(
        frame,
        x="Year",
        y="Life expectancy (years)",
        line_group="Country",
        facet_col="Continent",
        hover_name="Country",
        template="simple_white",
        width=640,
        height=420,
    )
    fig.update_traces(line={"width": 1})
    fig.for_each_annotation(lambda note: note.update(text=note.text.split("=")[-1]))
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "How did life expectancy change across countries in Asia and Europe?",
    "book": {"quote": "have a high rather than low data density", "page": 168, "line": 6793},
    "before": {
        "caption": "Two dials show only continent averages for 2007. The earlier "
        "observations and the differences among countries disappear.",
        "summary": "Life expectancy in Japan rose from about 63 to 83 years between "
        "1952 and 2007. In Bangladesh it rose from about 37 to 64 years.",
    },
    "after": {
        "caption": "The country histories show a general rise and more spread in Asia. "
        "Each line gives its country and values on hover.",
        "summary": "Life expectancy in Japan rose from about 63 to 83 years between "
        "1952 and 2007. In Bangladesh it rose from about 37 to 64 years.",
    },
}
