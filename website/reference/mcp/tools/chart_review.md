Use `chart_review` when the agent makes, changes or reviews a chart, as
[`xpd chart-review`](../../../commands/chart-review/) does, and it returns the same JSON. It lists
each chart call of the file with its kind and the plotly call that draws it, the plotly rules
that fire, and each question of Tufte's book that applies to a chart, with its page. With `run`,
the user confirms first, because the adapter runs project code.

## Example

```python title="sales.py"
import plotly.express as px


def sales_pie():
    return px.pie(names=["North", "South"], values=[60, 40], title="SALES BY REGION")
```

```json
{"path":"sales.py"}
```

The result holds `file`, `libraries`, `charts`, `issues`, `questions` and `facts`. These are its
charts and the rules that fire.

```json
{"file":"sales.py","libraries":["plotly"],"charts":[{"line":5,"call":"px.pie","kind":"pie","library":"plotly","plotly":"go.Table, or px.bar sorted by value"}],"issues":["..."],"questions":"...","facts":null}
```

## Errors

A path outside the project root fails before xpd runs. A file that cannot be read fails with the
binary's message. A run that fails, or that the user declines, leaves the questions in place.
