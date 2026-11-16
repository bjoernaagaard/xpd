"""Build a Plotly figure from made up rows without a saved figure or template file."""

import plotly.express as px

figure = px.scatter(
    x=[1, 2, 3], y=[2, 4, 3], labels={"x": "Day", "y": "Count"}, template="simple_white"
)
figure.update_layout(showlegend=False, title="Daily count")
figure.update_xaxes(title="Day", showgrid=False, zeroline=False)
figure.update_yaxes(title="Count", showgrid=False, zeroline=False)
result = figure.to_json()
print(
    f"traces={len(figure.data)}, points={len(figure.data[0].x)}, title={figure.layout.title.text}"
)
