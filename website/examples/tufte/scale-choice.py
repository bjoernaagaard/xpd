import plotly.graph_objects as go


def load_data():
    return [(year, 100 * 1.12 ** (year - 2010)) for year in range(2010, 2020)]


def value_table(data):
    return {"columns": ["Year", "Illustrative index"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    years, values = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=years, y=values, mode="lines+markers"))
    fig.update_layout(
        template="simple_white", width=1100, height=200, yaxis_range=[0, 1000], yaxis_title="Index"
    )
    return fig


def make_after(data) -> go.Figure:
    years, values = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=years, y=values, mode="lines+markers"))
    fig.update_layout(
        template="simple_white",
        width=560,
        height=430,
        yaxis_type="log",
        yaxis_title="Index (log scale)",
    )
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Illustrative stock index with 12 percent annual growth.",
    "question": "Did the index grow by the same proportion each year?",
    "book": {"quote": "échelle logarithmique", "page": 169, "line": 6825},
    "before": {
        "caption": "The wide linear chart compresses the series and makes early "
        "increases harder to see.",
        "summary": "The index grows 12 percent each year, from 100 in 2010 to about 277 in 2019.",
    },
    "after": {
        "caption": "On the log scale, equal vertical steps represent equal proportional "
        "gains. The points form a straight line.",
        "summary": "The index grows 12 percent each year, from 100 in 2010 to about 277 in 2019.",
    },
}
