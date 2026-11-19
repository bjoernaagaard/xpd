Tick labels turned away from the horizontal are harder to read. For long category names, put the categories on y with a horizontal bar chart.

```diff lang="python"
-fig = px.bar(df, x="answer", y="share")
-fig.update_xaxes(tickangle=-90)
+fig = px.bar(df, x="share", y="answer", orientation="h")
```

## What the rule reports

The rule reports a `tickangle` written as a number other than 0, 180 and 360, on an x or y axis of a plotly figure. It reads the keyword in `update_xaxes`, `update_yaxes`, `go.layout.XAxis` and `go.layout.YAxis`, the underscore form such as `xaxis_tickangle` in `update_layout` and `go.Layout`, and `xaxis=dict(tickangle=...)` or `xaxis={"tickangle": ...}` in `update_layout`, `go.Layout` and `go.Figure`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The book lists as friendly "words run from left to right, the usual direction for reading
occidental languages", and as unfriendly "words run vertically, particularly along the Y-axis;
words run in several different directions" (page 183). plotly keeps a `tickangle` as written.
`update_xaxes(tickangle=-90)` resolved to -90, and both category labels of a bar chart were drawn
rotated. With the categories on y, as in `px.bar(orientation="h")`, every tick label was
drawn level.

The rule has no fix, because `tickangle=0` can make long labels run into each other, and the
level layout swaps x and y. An angle given by a name is left alone, and so is the `tickangle`
of a colorbar.
