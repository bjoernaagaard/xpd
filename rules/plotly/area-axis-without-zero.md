An area chart whose value axis starts above zero draws fills whose height no longer follows their value. Let the axis start at 0, or draw lines instead of filled areas.

```diff lang="python"
-fig = px.area(df, x="month", y="sales", range_y=[90, 101])
+fig = px.area(df, x="month", y="sales")
+line = px.line(df, x="month", y="sales", range_y=[90, 101])
```

## What the rule reports

The rule reports an area chart whose value axis is given a range that starts above zero: `range_y` on `px.area`, and on a figure of filled areas `update_yaxes(range=...)`, `update_layout(yaxis_range=...)` and `update_layout(yaxis=dict(range=...))`.

## How to fix it

`xpd check --fix --unsafe-fixes` resets the lower bound of the explicit range list to `0` (for example `[90, 101]` -> `[0, 101]`). It is unsafe because switching to `px.line` or dropping the range altogether may be preferred.

An area chart shows its value by the height of the fill below the line. plotly starts the axis
of `px.area` and of `go.Scatter(fill="tozeroy")` at zero by itself. It keeps an explicit range
as written and cuts the fill at the lower edge of the plot, a Lie Factor of 19 as for a bar.

| Values 95 and 100 | Range | Fill heights in pixels |
| --- | --- | --- |
| `px.area` | 0 to 105.26, chosen by plotly | 324.9 and 342.0, the ratio of the values |
| `[90, 101]` set in `px.area`, in `update_yaxes` or in `update_layout` | 90 to 101 | 163.6 and 327.3 |
| `px.line` | 94.72 to 100.28, chosen by plotly | no fill |

A line chart needs no zero, so the replacement draws the narrow range with `px.line`.

The rule reads a range as [`bar-axis-without-zero`](bar-axis-without-zero.md) does. Both ends must
be written numbers above zero, and the lower end must have as many digits before the point as
the upper end, or one fewer. A call with `orientation` or `log_y` is left alone, and a range set on the
figure afterwards is left alone in a file that names a log axis. A figure of filled areas is
`px.area` without `orientation` and `log_y`, or a `go.Figure(...)` whose own arguments hold a
`go.Scatter` with `fill="tozeroy"`.
