`mirror=True` repeats the axis line on the far side of the plot, which closes a box around the data and adds ink that shows nothing. Drop `mirror`, and keep the two axis lines.

```diff lang="python"
 fig = px.scatter(df, x="day", y="tip")
-fig.update_xaxes(showline=True, mirror=True)
-fig.update_yaxes(showline=True, mirror=True)
+fig.update_xaxes(showline=True)
+fig.update_yaxes(showline=True)
```

## What the rule reports

The rule reports `mirror=True`, `mirror="all"`, `mirror="ticks"` or `mirror="allticks"` beside `showline=True` on an x or y axis of a plotly figure, in `update_xaxes`, `update_yaxes`, `go.layout.XAxis`, `go.layout.YAxis`, and in `xaxis=dict(...)` or `xaxis={...}` in `update_layout`, `go.Layout` and `go.Figure`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix drops `mirror`, so the box around the plot is open.

The book redraws a chart until it has "only the frame lines remaining as pure non-information"
(page 103), and says that a frame which shows the range of the data "should replace the
non-data-bearing frame" (page 131). `mirror=True` draws each axis line a second time on the
far side of the plot, which closes a box around the data. The x axis line had one segment with
`showline=True` alone and two with `mirror=True`. Under the default template `mirror=True`
without `showline` drew no line, so the rule reads the two together.
