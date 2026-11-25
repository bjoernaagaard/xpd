Use `xpd chart-review` when you make, change or review a chart, in plotly, Dash, matplotlib,
seaborn or pandas. It finds each chart call of a Python file and gives it a kind and the plotly
call that draws it, prints the plotly rules that fire on the file, and asks each question of
Edward Tufte's book "The Visual Display of Quantitative Information" that applies to a chart,
with its page. The [chart review guide](../../guides/chart-review/) shows how an agent uses it
and lists every question.

## Examples

```python title="report.py"
import matplotlib.pyplot as plt
import seaborn as sns


def revenue(frame):
    fig, ax = plt.subplots()
    sns.barplot(frame, x="region", y="revenue", ax=ax)
    return fig
```

```console
$ xpd chart-review report.py --format json | jq -c '{libraries, charts, questions: (.questions | length)}'
{"libraries":["matplotlib","seaborn"],"charts":[{"call":"sns.barplot","kind":"bar","library":"seaborn","line":7,"plotly":"px.bar"}],"questions":36}
```

`--run <adapter.py>` adds what the drawn figures measure. The adapter defines `figures()`, which
returns a dict of name to figure, and it runs in the project's Python.

```python title="report_figures.py"
import pandas as pd

from report import revenue


def figures():
    frame = pd.DataFrame({"region": ["North", "South", "West"], "revenue": [9.0, 6.0, 4.0]})
    return {"revenue": revenue(frame)}
```

```console
$ xpd chart-review report.py --run report_figures.py --format json | jq -c '.questions[] | select(.id == "small-data-table") | .measured'
[{"fact":"values","figure":"revenue","findings":["3 values drawn, 20 or fewer, which a table or a sentence can show"],"summary":"3 values drawn: bars 0 3 bar"}]
```

Without a file, or with `--questions`, it prints every question.

```console
$ xpd chart-review --questions --format json | jq '.questions | length'
46
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | It printed a review, also when a run failed, which the review reports under `facts.error` |
| 2 | It could not run, as when the file cannot be read |
