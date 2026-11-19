A spline line can pass through values the data does not hold. Draw straight segments between the points.

```diff lang="python"
-fig = px.line(df, x="hour", y="reading", line_shape="spline")
+fig = px.line(df, x="hour", y="reading", line_shape="linear")
```

## What the rule reports

The rule reports the line shape `"spline"`.

| Setting | Calls |
| --- | --- |
| `line_shape="spline"` | `px.area`, `px.line`, `px.line_polar`, `px.line_ternary`, `update_traces` on a plotly figure, and the five go traces whose line takes the shape: `go.Scatter`, `go.Scattercarpet`, `go.Scatterpolar`, `go.Scattersmith`, `go.Scatterternary` |
| `line=dict(shape="spline")`, `line={"shape": "spline"}` | the same five go traces |

The rule reports the setting and does not prove that the trace draws a line. A trace with only
markers draws no curve, so the advice applies only when the trace draws lines. The unsafe fix
changes the shape also when no line uses it.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `"linear"`.

"Above all else show the data" (page 92). A spline draws a smooth curve through the points,
and the curve passes through values that the data does not hold. plotly computes the spline in
pixels, so how far the curve leaves the data depends on the data, on the axis range and on the
size of the figure. Because the distance varies, the rule is info.

| Readings 0, 0, 10, 10, 0, 0, drawn at 700 by 500 pixels | Highest point drawn | Lowest point drawn |
| --- | --- | --- |
| a straight line | 10 | 0 |
| `go.Scatter(line_shape="spline")`, y range chosen by plotly | 10.55 | -0.37 |
| `px.line(line_shape="spline")`, y range chosen by plotly | 10.51 | -0.34 |
| `go.Scatter(line_shape="spline")`, y axis set to -5 to 15 | 10.8 | -0.53 |
| `px.line(line_shape="spline")`, y axis set to -5 to 15 | 10.75 | -0.5 |

The fix is unsafe, because straight segments make the figure look different. It leaves a
`smoothing` argument in place, which plotly ignores for a straight line. A shape given by a
name is left alone.
