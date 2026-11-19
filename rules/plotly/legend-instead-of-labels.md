A legend for a few lines makes the reader look back and forth between the legend and the lines. Write each name at the end of its line, and hide the legend.

```diff lang="python"
 fig = go.Figure(data=[
-    go.Scatter(x=years, y=north, name="north"),
-    go.Scatter(x=years, y=south, name="south"),
+    go.Scatter(x=years, y=north, mode="lines+text", text=["", "", "north"], textposition="middle right"),
+    go.Scatter(x=years, y=south, mode="lines+text", text=["", "", "south"], textposition="middle right"),
 ])
+fig.update_layout(showlegend=False)
```

## What the rule reports

The rule reports a figure of two to four named lines with a legend, in a file that names no `showlegend`, no annotation and no text mode. The figure is one of three forms:

- a `go.Figure` whose data is a written list of two to four `go.Scatter(...)` calls, each with a `name`
- two to four such calls added one by one with `fig.add_trace(...)` to an empty `go.Figure()`
- `px.line` with `color="column"`, or with `y` as a written list of two to four column names

## How to fix it

xpd has no automatic fix, so make the change by hand.

The book asks "to integrate the caption and legend into the design so that the eye is not
required to dart back and forth between textual material and the graphic" (page 180), and lists
as friendly "labels are placed on the graphic itself; no legend is required" (page 183).

| Figure | `showlegend` | What it drew |
| --- | --- | --- |
| `go.Figure` with two named traces | `True` | a legend of two entries |
| two named lines added one by one with `add_trace` | `True` | a legend of two entries, the same figure |
| `px.line(color="sex")` | not recorded | two lines and a legend of two entries |
| `px.line(y=["a", "b"])` | not recorded | a legend of two entries titled "variable" |
| `mode="lines+text"`, a label at the last point of each line, and `showlegend=False` | `False` | no legend, and the names beside the lines |

`add_trace` statements count when they follow each other with no second assignment of the name
in between. The number of values of a column is known only at run time, so a `px.line` with
`color` may draw many lines. The rule leaves alone the forms in the table.

| Form | Why |
| --- | --- |
| a figure that also gets a trace in another way, such as `add_bar`, `add_traces`, an `add_trace` of another trace, or an `add_trace` in a loop or a condition | the code does not show the size of the legend |
| an `add_trace` with `row`, `col` or `secondary_y` | the traces are in different cells or on different axes |
| a call with `line_group` | many lines share one entry |
| a trace with `mode="markers"`, or a file that calls `update_traces(mode="markers")` | there is no line to label |
| a written list with a fifth trace, or a run of five `add_trace` calls | many lines have no room for a label each |
