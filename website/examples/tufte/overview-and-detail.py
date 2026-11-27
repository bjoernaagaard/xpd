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
    means = frame.groupby(["Continent", "Year"], as_index=False)["Life expectancy (years)"].mean()
    return px.line(
        means,
        x="Year",
        y="Life expectancy (years)",
        color="Continent",
        line_dash="Continent",
        template="simple_white",
    )


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
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "How did life expectancy change across countries in Asia and Europe?",
    "book": {"quote": "from a broad overview to the fine structure", "page": 17, "line": 114},
    "before": {
        "caption": "Two lines show continent averages. The averages hide differences "
        "in starting values and in the pace of change.",
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
