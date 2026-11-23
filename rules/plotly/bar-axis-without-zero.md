A bar chart whose value axis starts above zero draws bars whose length no longer follows their value. Let the axis start at 0, or draw dots instead of bars.

```diff lang="python"
-fig = px.bar(df, x="team", y="score", range_y=[90, 101])
+fig = px.bar(df, x="team", y="score")
```

## What the rule reports

The rule reports a bar chart whose value axis is given a range that starts above zero.

| Figure | Range settings read |
| --- | --- |
| `px.bar`, `px.histogram` | `range_y` |
| `px.bar(orientation="h")` | `range_x` |
| a figure of upright bars, afterwards | `update_yaxes(range=...)`, `update_layout(yaxis_range=...)`, `update_layout(yaxis=dict(range=...))` |
| `go.Figure(data=[go.Bar(...)], layout=...)` | `yaxis=dict(range=...)` in the layout, or `layout_yaxis_range=...` |
| a `go.Figure()` that gets its bars from `fig.add_trace(go.Bar(...))` or `fig.add_bar(...)` | the same three settings afterwards |

## How to fix it

`xpd check --fix --unsafe-fixes` resets the lower bound of the explicit range list to `0` (for example `[90, 101]` -> `[0, 101]`). It is unsafe because switching to `px.scatter` or dropping the range altogether may be preferred.

A bar shows its value by its length. plotly starts a bar axis at zero by itself and keeps an
explicit range as written. With `[90, 101]` the data grows by 5.3 percent and the bar by 100
percent, a Lie Factor of 19.

| Bars for 95 and 100 | Range | Lengths in pixels |
| --- | --- | --- |
| `px.bar` | 0 to 105.26, chosen by plotly | 324.9 and 342.0, the ratio of the values |
| `px.bar(range_y=[90, 101])` | 90 to 101 | 163.6 and 327.3 |
| `px.histogram` of counts 95 and 100, `range_y=[90, 101]` | 90 to 101 | 163.6 and 327.3 |
| lying bars, `range_x=[90, 101]` | 90 to 101 | 245.4 and 490.9 |

When the differences are the message, `px.scatter` shows each value by its position, which
needs no zero.

The rule reads a range only when both ends are written numbers above zero and the lower end
has as many digits before the point as the upper end, or one fewer. For example, `[90, 101]`
and `[2, 5]` are read, and `[1, 100]`, `[0, 101]`, `[-10, 101]` and `[low, high]` are not. A call with
`log_y` is left alone, and a range set on the figure afterwards is left alone in a file that
names a log axis. A figure of upright bars is `px.bar` or `px.histogram` without `orientation`
and `base`, or a `go.Figure(...)` whose own arguments hold a `go.Bar` without `orientation` and
`base`. With `base` each bar starts at its base and not at zero, so bars from 91 to 95 and from
93 to 100 lay whole inside `range_y=[90, 101]`, with lengths in the ratio of their values. A
`go.Figure` with a `go.Bar` and `layout=go.Layout(yaxis=dict(range=[90, 101]))` resolved to the
y range 90 to 101, as `update_yaxes` does.

The rule also reads bars added to an empty `go.Figure()`, and plotly kept their range as
written.

| Bars of 95 and 100 in a `go.Figure` | Lengths in pixels |
| --- | --- |
| added with `add_trace`, without a range | 288.8 and 304.0 |
| added with `add_trace`, then `update_yaxes(range=[90, 101])` | 145.4 and 290.9 |
| added with `add_bar`, then `update_layout(yaxis_range=[90, 101])` | 145.4 and 290.9 |
| given to `go.Figure` itself, with the range | 145.4 and 290.9 |

The bar must be added to the same variable after its assignment and before the range, in the
statement itself or in a loop or a condition, with no second assignment of the name in between.
A bar with `orientation`, `base` or `yaxis` is left alone.
