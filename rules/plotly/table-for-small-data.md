A bar chart of a few numbers written in the code shows them as bar lengths and writes none of them. Show them in a table with `go.Table`.

```diff lang="python"
-fig = px.bar(x=["north", "south", "east", "west"], y=[41.3, 37.8, 12.6, 8.3])
+fig = go.Figure(go.Table(
+    header=dict(values=["region", "share"]),
+    cells=dict(values=[["north", "south", "east", "west"], [41.3, 37.8, 12.6, 8.3]]),
+))
```

## What the rule reports

The rule reports `px.bar` and `go.Bar` whose values, `y`, or `x` with `orientation="h"`, are a written list or tuple of 1 to 20 numbers, and `px.pie` and `go.Pie` whose `values` is one, when the call is a px call or the only trace of a `go.Figure`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

"Tables usually outperform graphics in reporting on small data sets of 20 numbers or less"
(page 56), and "tables are preferable to graphics for many small data sets" (page 178). A
chart of a few numbers draws them as lengths or angles and does not write them.

| Chart | What it wrote |
| --- | --- |
| `px.bar` of 41.3, 37.8, 12.6 and 8.3 | the four names and the ticks 0 to 40, and none of the four values |
| `go.Pie` of 413, 378, 126 and 83 | the four shares as percentages, and none of the values |
| `go.Table` of the same values | all four values |

xpd cannot count the rows of a data frame or the traces of a figure. The rule therefore reads
only a px call or a `go.Figure` whose data is one trace, where the code shows the whole data. A pie is also reported by [`pie-chart`](pie-chart.md), for its form. The rule leaves
alone the forms in the table.

| Form | Why |
| --- | --- |
| a trace added with `add_trace` | the code does not show the other traces |
| an `x` of numbers on a bar without `orientation="h"` | the numbers are positions |
| a chart that writes its values with `text`, `texttemplate`, `textinfo` or `text_auto`, or a file that names `textposition` | the values are already written |
| a list with a name, a `None` or a 21st number | the list is not 1 to 20 written numbers |
