import plotly.graph_objects as go


def load_data():
    return [
        ("Day train", 8, 0),
        ("Day train", 12, 240),
        ("Day train", 15, 460),
        ("Overnight train", 20, 0),
        ("Overnight train", 24, 240),
        ("Overnight train", 28, 460),
    ]


def value_table(data):
    return {
        "columns": ["Train", "Hour since start of service day", "Distance from Paris (km)"],
        "rows": [list(row) for row in data],
    }


def make_before(data) -> go.Figure:
    fig = go.Figure()
    for train, dash in (("Day train", "solid"), ("Overnight train", "dash")):
        hours, distance = zip(
            *((hour, km) for name, hour, km in data if name == train), strict=True
        )
        display_hours = [hour % 24 for hour in hours]
        if train == "Overnight train":
            display_hours = [20, 24, None, 0, 4]
            distance = [0, 240, None, 240, 460]
        fig.add_trace(
            go.Scatter(
                x=display_hours, y=distance, name=train, mode="lines+markers", line={"dash": dash}
            )
        )
    fig.update_layout(
        template="simple_white",
        xaxis_title="Hour of service",
        yaxis_title="Distance from Paris (km)",
        xaxis_tickvals=[0, 4, 8, 12, 16, 20, 24],
    )
    return fig


def make_after(data) -> go.Figure:
    fig = go.Figure()
    for train, color, dash in (
        ("Day train", "#3b82c4", "solid"),
        ("Overnight train", "#d97706", "dash"),
    ):
        hours, distance = zip(
            *((hour, km) for name, hour, km in data if name == train), strict=True
        )
        fig.add_trace(
            go.Scatter(
                x=hours,
                y=distance,
                name=train,
                mode="lines+markers",
                line={"width": 2, "color": color, "dash": dash},
                marker={"size": 7, "color": color},
                hovertemplate="%{fullData.name}: %{y} km at hour %{x}<extra></extra>",
            )
        )
    fig.add_vline(x=24, line_width=1, line_dash="dot", line_color="#9ca3af")
    fig.update_layout(
        template="simple_white",
        font={"family": "Inter, system-ui, -apple-system, sans-serif", "color": "#1f2937"},
        title={
            "text": "Paris to Lyon service schedule",
            "x": 0,
            "xanchor": "left",
            "subtitle": {
                "text": "Repeating four hours past midnight keeps the overnight trip intact"
            },
        },
        xaxis_title="Clock time (repeated past midnight)",
        yaxis_title="Distance from Paris (km)",
        xaxis_tickvals=[8, 12, 16, 20, 24, 28],
        xaxis_ticktext=["08:00", "12:00", "16:00", "20:00", "00:00", "04:00"],
        yaxis_range=[-20, 500],
        legend={"orientation": "h", "y": 1.04, "x": 0, "xanchor": "left"},
        margin={"l": 64, "r": 24, "t": 92, "b": 64},
    )
    return fig


EXAMPLE = {
    "source": "Illustrative Paris to Lyon schedule based on the design of the schedule "
    "reproduced in Tufte's book.",
    "question": "When do the overnight trains reach Lyon?",
    "book": {"quote": "parts of the cycle should be repeated", "page": 100, "line": 3859},
    "before": {
        "caption": "The overnight line ends at midnight and resumes at the far left. "
        "The arrival belongs to the same trip.",
        "summary": "The overnight train leaves Paris at 20:00, is 240 km away at "
        "midnight and reaches Lyon at 04:00 the next day.",
    },
    "after": {
        "caption": "Repeating the first four hours beyond midnight joins all three "
        "overnight points.",
        "summary": "The overnight train leaves Paris at 20:00, is 240 km away at "
        "midnight and reaches Lyon at 04:00 the next day.",
    },
}
