import plotly.graph_objects as go


def load_data():
    return list(zip(range(1951, 1960), [265, 230, 275, 245, 325, 290, 285, 250, 245], strict=True))


def value_table(data):
    return {"columns": ["Year", "Traffic deaths"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    years, deaths = zip(*data[4:6], strict=True)
    fig = go.Figure(go.Scatter(x=years, y=deaths, mode="lines+markers"))
    fig.update_layout(
        template="simple_white", title="Connecticut traffic deaths", yaxis_title="Traffic deaths"
    )
    return fig


def make_after(data) -> go.Figure:
    years, deaths = zip(*data, strict=True)
    fig = go.Figure(go.Scatter(x=years, y=deaths, mode="lines+markers"))
    fig.update_layout(
        template="simple_white", title="Connecticut traffic deaths", yaxis_title="Traffic deaths"
    )
    fig.add_vline(x=1955.5, line_dash="dash", annotation_text="Stricter enforcement")
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Connecticut traffic deaths transcribed from the chart in Tufte's book. Values "
    "are approximate.",
    "question": "How did the change after stricter speed enforcement compare with nearby years?",
    "book": {"quote": "Graphics must not quote data out of context", "page": 78, "line": 2968},
    "before": {
        "caption": "Two years show a fall after stricter enforcement and give no "
        "comparison with earlier changes.",
        "summary": "Deaths fell from about 325 in 1955 to 290 in 1956. The earlier "
        "counts ranged from about 230 to 275.",
    },
    "after": {
        "caption": "Nine years show that 1955 was unusually high. The series alone "
        "cannot establish the effect of enforcement.",
        "summary": "Deaths fell from about 325 in 1955 to 290 in 1956. The earlier "
        "counts ranged from about 230 to 275.",
    },
}
