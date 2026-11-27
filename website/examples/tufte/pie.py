import plotly.express as px
import plotly.graph_objects as go


def load_data():
    votes = px.data.election().iloc[0]
    return [(candidate, int(votes[candidate])) for candidate in ("Coderre", "Bergeron", "Joly")]


def value_table(data):
    return {"columns": ["Candidate", "Votes"], "rows": [list(row) for row in data]}


def make_before(data) -> go.Figure:
    names, votes = zip(*data, strict=True)
    fig = go.Figure(
        go.Pie(labels=names, values=votes, marker={"colors": ["#245a80", "#80547b", "#b98442"]})
    )
    fig.update_layout(template="simple_white")
    return fig


def make_after(data) -> go.Figure:
    names, votes = zip(*sorted(data, key=lambda row: row[1]), strict=True)
    fig = go.Figure(go.Bar(x=votes, y=names, orientation="h", text=votes, textposition="outside"))
    fig.update_layout(template="simple_white", xaxis_title="Votes", xaxis_range=[0, 3500])
    fig.update_layout(
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        colorway=["#3b82c4", "#d97706", "#14967f", "#8b5cf6", "#dc4c3e", "#78716c"],
    )
    return fig


EXAMPLE = {
    "source": "Plotly Express Montreal election dataset, Bois-de-Liesse district.",
    "question": "Who received the most votes in Bois-de-Liesse?",
    "book": {
        "quote": "Tables are clearly the best way to show exact numerical values",
        "page": 178,
        "line": 7166,
    },
    "before": {
        "caption": "The pie uses angles to show each share. The angles and percentages "
        "do not give vote counts.",
        "summary": "Joly received 3,024 votes, Coderre 2,481 and Bergeron 1,829.",
    },
    "after": {
        "caption": "The bars start at zero and print the vote counts. Joly received the "
        "most votes.",
        "summary": "Joly received 3,024 votes, Coderre 2,481 and Bergeron 1,829.",
    },
}
