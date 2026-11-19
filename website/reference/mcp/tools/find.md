Use `find` before choosing a library or a function for a task, as
[`xpd find`](../../../commands/find/) does. It searches the index in the xpd binary, so it works for a
library that the project has not installed. Read an entry it returns with [`api`](../api/)
before writing code. `find` runs in the server's working folder, because it reads no project
file.

## Example

```json
{"task":"rounded bars","kind":"recipe","limit":1}
```

The result has `symbols`, `rules` and `recipes`. This is the one recipe, without its code.

```json
{"kind":"recipe","package":"plotly","name":"plotly/recipe/rounded-bars","callable":null,"rules":[],"recipe":{"title":"Rounded Bars","symbols":["plotly.data.medals_wide","plotly.graph_objects.Bar","plotly.subplots.make_subplots"],"source":"https://github.com/plotly/plotly.py/blob/v7.1.0/doc/python/bar-charts.md#L746","license":"MIT","modified":false},"signature":null}
```

## Errors

No match gives empty lists. The first call can take a few minutes, because it downloads the
333 MB search model.
