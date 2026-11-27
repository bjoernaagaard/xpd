import plotly.express as px
import plotly.graph_objects as go


def load_data():
    frame = px.data.gapminder()
    selected = frame[frame["year"].isin([1952, 1972, 1992, 2007])]
    return list(
        selected[["country", "year", "gdpPercap", "lifeExp", "continent"]].itertuples(
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
    fig = px.scatter(
        frame,
        x="Income per person",
        y="Life expectancy",
        animation_frame="Year",
        animation_group="Country",
        hover_name="Country",
        log_x=True,
        range_x=[200, 150000],
        range_y=[20, 90],
        template="simple_white",
    )
    fig.update_layout(xaxis_title="Income per person", yaxis_title="Life expectancy (years)")
    return fig


def make_after(data) -> go.Figure:
    import pandas as pd

    frame = pd.DataFrame(
        data, columns=["Country", "Year", "Income per person", "Life expectancy", "Continent"]
    )
    fig = px.scatter(
        frame,
        x="Income per person",
        y="Life expectancy",
        facet_col="Year",
        facet_col_wrap=2,
        hover_name="Country",
        log_x=True,
        range_x=[200, 150000],
        range_y=[20, 90],
        template="simple_white",
        height=550,
        width=700,
    )
    fig.update_xaxes(title_text=None, nticks=3)
    fig.update_yaxes(title_text=None, nticks=4)
    fig.update_xaxes(title_text="Income per person", row=1)
    fig.update_yaxes(title_text="Life expectancy (years)", col=1)
    fig.update_layout(margin={"l": 62, "r": 20, "t": 38, "b": 62})
    fig.for_each_annotation(lambda note: note.update(text=note.text.split("=")[-1]))
    return fig


EXAMPLE = {
    "source": "Plotly Express Gapminder dataset.",
    "question": "How did life expectancy and income change together over four snapshots?",
    "book": {"quote": "Small multiples resemble the frames of a movie", "page": 170, "line": 6842},
    "before": {
        "caption": "The animation shows one year at a time. Comparing a country across "
        "years requires remembering earlier frames.",
        "summary": "Life expectancy reached 70 years in 5 countries in 1952, 30 in 1972, 54 in 1992 and 83 in 2007.",
    },
    "after": {
        "caption": "Four panels use the same scales, so a reader can compare years "
        "directly. More countries reach 70 years by 2007.",
        "summary": "Life expectancy reached 70 years in 5 countries in 1952, 30 in 1972, 54 in 1992 and 83 in 2007.",
    },
}
