A pie chart makes the reader compare angles, with no numbers on a common scale. Show the shares as a sorted bar chart, or as a table when they are few.

```diff lang="python"
-fig = px.pie(df, names="day", values="tip")
-several = px.pie(df, names="day", values="tip", facet_col="shop")
+fig = px.bar(df.sort_values("tip"), x="tip", y="day", orientation="h")
+several = px.bar(df, x="tip", y="day", orientation="h", facet_col="shop")
```

## What the rule reports

The rule reports `px.pie(...)` and `go.Pie(...)`. A second message, for several pies, goes to the forms in the table.

| Form | Example |
| --- | --- |
| `px.pie` with `facet_col` or `facet_row` | `px.pie(df, names="day", values="tip", facet_col="shop")` |
| two `go.Pie` in the data list of `go.Figure` or of `add_traces` | `go.Figure([go.Pie(...), go.Pie(...)])` |
| an `add_trace` of a pie next to another `add_trace` of a pie on the same figure | `fig.add_trace(go.Pie(...), row=1, col=1)` twice |
| a pie added in a loop to a cell that changes with the loop | `fig.add_trace(go.Pie(...), row=1, col=i)` in `for i in ...` |

## How to fix it

xpd has no automatic fix, so make the change by hand.

"A table is nearly always better than a dumb pie chart; the only worse design than a pie chart
is several of them, for then the viewer is asked to compare quantities located in spatial
disarray both within and between pies. Given their low data-density and failure to order
numbers along a visual dimension, pie charts should never be used" (page 178). For a few
shares, `go.Table` shows the exact numbers.

| Code | What plotly made of it |
| --- | --- |
| `px.pie`, `go.Pie`, with and without `hole` | a trace of type `pie` |
| `px.pie(facet_col="sex")` | two pies side by side |
| `px.pie(facet_col="sex", facet_row=...)` | four pies |
| `make_subplots(rows=1, cols=2)` with two cells of type `"domain"` and a `go.Pie` added to each | two traces of type `pie`, with the domains 0 to 0.45 and 0.55 to 1 across |

Each `go.Pie` call is reported once. It gets the message for several pies when it is in a written
`go.Figure` data list, or when the figure it is added to is proved to be a plotly figure. When the
object that several pies are added to is not proved to be a figure, each pie keeps the first
message. Two `add_trace` calls count as one figure only when no second assignment of the name
comes between them.

A facet given as `None` draws one pie and gets the first message. `px.sunburst` has its own
rule, [`sunburst-as-pie`](sunburst-as-pie.md). `px.funnel_area` is not reported, because its
sections were drawn with areas in the ratio of the values, stacked from top to bottom.
