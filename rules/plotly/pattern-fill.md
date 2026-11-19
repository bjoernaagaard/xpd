Hatching groups with `pattern_shape` makes the chart shimmer, which Tufte calls the most common kind of clutter. Tell groups apart with shades of one color, or label them on the chart.

```diff lang="python"
-fig = px.bar(df, x="day", y="tip", pattern_shape="shop")
+fig = px.bar(df, x="day", y="tip", color="shop", color_discrete_sequence=["#555", "#aaa"])
```

## What the rule reports

The rule reports hatching.

| Setting | Calls |
| --- | --- |
| `pattern_shape="column"` | `px.area`, `px.bar`, `px.bar_polar`, `px.histogram`, `px.timeline` |
| a pattern shape on `marker.pattern` | `go.Bar`, `go.Barpolar`, `go.Funnelarea`, `go.Histogram`, `go.Icicle`, `go.Pie`, `go.Sunburst`, `go.Treemap` |
| `fillpattern` | `go.Scatter` |

## How to fix it

xpd has no automatic fix, so make the change by hand.

"This moiré vibration, probably the most common form of graphical clutter, is inevitably bad
art and bad data graphics" (page 108), and "cross-hatching should be replaced with tint screens
of shades of gray" (page 111). `px.bar(pattern_shape="sex")` resolved the pattern shapes of its
two traces to `""` and `"/"`, and the drawn figure held two pattern images, where the same
chart without the argument held none. `go.Bar(marker_pattern_shape="/")` and
`go.Scatter(fillpattern=dict(shape="x"))` each drew one.

On a go trace the rule reads `marker_pattern_shape=`, `marker=dict(pattern=dict(shape=...))`,
`fillpattern_shape=` and `fillpattern=dict(shape=...)`, with `dict(...)` or `{...}`. A shape of
`""` draws no pattern and is left alone, and so is a shape given by a name.
