`showgrid=True` without a muted color draws a grid line at every tick that competes with the data. Leave the grid out, or give it a light `gridcolor` and a thin `gridwidth`.

```diff lang="python"
 fig = px.scatter(df, x="day", y="tip")
-fig.update_xaxes(showgrid=True)
-fig.update_yaxes(gridcolor="black")
+fig.update_yaxes(showgrid=True, gridcolor="#eee")
```

## What the rule reports

The rule reports `showgrid=True` without `gridcolor` and `gridwidth` beside it, and `gridcolor` set to black, on an x or y axis of a plotly figure. It reads the keyword in `update_xaxes`, `update_yaxes`, `go.layout.XAxis` and `go.layout.YAxis`, the underscore form such as `xaxis_showgrid` in `update_layout` and `go.Layout`, and `xaxis=dict(showgrid=True)` or `xaxis={"showgrid": True}` in `update_layout`, `go.Layout` and `go.Figure`.

The rule leaves alone a call that sets `gridcolor` or `gridwidth` beside `showgrid=True`,
because the call has chosen its grid. It reads black as `"black"`, `"#000"`, `"#000000"` and
`"rgb(0,0,0)"`, in upper or lower case, because how dark another color is against the
background cannot be read from the code. A black `gridcolor` beside `showgrid=False` draws
nothing and is left alone, and so are the lines of latitude and longitude on a map. The fix is unsafe, because it removes grid lines from a figure
drawn with `"simple_white"`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

unsafe for `showgrid=True`: `showgrid=False`. unsafe for a black grid: `"#eee"`.

"The grid should usually be muted or completely suppressed so that its presence is only
implicit" (page 112), and "dark grid lines are chartjunk" (page 113). `showgrid=True` either
does nothing or brings back the ink that the template left out.

| Template of a scatter plot | Grid lines without `showgrid=True` | Grid lines with it |
| --- | --- | --- |
| `"plotly"` | 10 | 10 |
| `"none"` | 10 | 10 |
| `"simple_white"` | 0 | 10, in `rgb(232,232,232)` |
