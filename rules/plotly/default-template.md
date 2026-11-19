plotly's default template tints the plot area and draws a grid, which is ink that shows no data. Set a plain template such as `"simple_white"` once.

```diff lang="python"
 import plotly.express as px
+import plotly.io as pio
+
+pio.templates.default = "simple_white"
 
 fig = px.scatter(df, x="day", y="tip")
-gray = px.scatter(df, x="day", y="tip", template="ggplot2")
+gray = px.scatter(df, x="day", y="tip", template="simple_white")
```

## What the rule reports

The rule reports a figure drawn with a template that tints the plot area and draws a grid.

| Form | Where it is reported |
| --- | --- |
| A file draws px or go figures with a plot area and never names a template or a `plot_bgcolor`. | once, on the first import of `plotly.express`, or on the first import of `plotly.graph_objects` when the file does not import `plotly.express` |
| A file draws through pandas, imports no plotly module, and calls `line`, `bar`, `barh`, `area`, `scatter`, `box` or `hist` on `df.plot`. | on the statement that makes plotly the plotting backend |
| The template `"plotly"`, `"ggplot2"` or `"seaborn"` is written as a string in a px call, in `go.Layout`, in `go.Figure(layout_template=...)`, in `update_layout` on a plotly figure, or assigned to `pio.templates.default`. | on the string |

The rule does not report seven templates. `"plotly_white"`, `"none"`, `"presentation"`,
`"gridon"`, `"xgridoff"` and `"ygridoff"` resolved to a white plot area, and `"plotly_dark"` is a
dark theme. Through the plotting backend of pandas, `df.plot.scatter(x="score", y="cost")` was equal,
as JSON, to `px.scatter(df, x="score", y="cost")`, also with `template="simple_white"`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

unsafe for a named template: `"simple_white"`. None for a file that sets no template.

plotly draws with the template `"plotly"` unless it is told otherwise. The book says that "the
grid should usually be muted or completely suppressed" (page 112).

| Template of a scatter plot | Plot background | Grid lines drawn | Axis lines |
| --- | --- | --- | --- |
| `"plotly"`, the default | `#E5ECF6` | 10, white | none |
| `"ggplot2"` | `rgb(237,237,237)` | 10, white | not recorded |
| `"seaborn"` | `rgb(234,234,242)` | 10, white | not recorded |
| `"simple_white"` | white, with `showgrid=False` | 0 | yes, with outside ticks |

The first form is silent in the cases in the table.

| Case | Why |
| --- | --- |
| a file that names `streamlit` | `st.plotly_chart` draws with the theme of Streamlit unless `theme=None` is passed |
| a file whose only figures have no plot area: `px.pie`, `px.treemap`, `px.sunburst`, `px.icicle`, `px.funnel_area`, `px.parallel_coordinates`, `px.parallel_categories`, `go.Pie`, `go.Treemap`, `go.Sunburst`, `go.Icicle`, `go.Funnelarea`, `go.Parcoords`, `go.Parcats`, `go.Sankey`, `go.Table`, `go.Indicator` | they drew no tint and no grid line with the default template |
| a module that the file imports sets `pio.templates.default = ...` | a scatter plot in a module of a package whose `__init__.py` assigns `pio.templates.default = "simple_white"` resolved to a white plot area with no grid, and so did one in a script that imports the module with the assignment |
| a module that the file imports sets `px.defaults.template = ...`, and the file draws only px figures | the setting changed the px figure and left a `go.Figure` at `#E5ECF6` |

`hovertemplate` and `texttemplate` are other attributes and do not count as a template, and a
scatter plot with a `hovertemplate` still resolved to `#E5ECF6`. To find a template set in
another module, xpd reads the `__init__.py` of each package the file is in, each module of the
project that the file imports, and each module that an imported module imports in turn. It
does not read a module that imports the file, such as the script that starts the program. xpd
therefore still reports a file beside the one with the assignment when the file does not import
it. The fix is unsafe, because
`"simple_white"` also changes the colors of the marks.
