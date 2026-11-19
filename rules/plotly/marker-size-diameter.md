plotly reads marker size as a diameter by default, so the area of a marker grows with the square of the value. Add `sizemode="area"` to the marker.

```diff lang="python"
-fig = go.Figure(go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=df["pop"])))
+sizeref = 2 * df["pop"].max() / 40**2
+fig = go.Figure(go.Scatter(
+    x=df["area"], y=df["rain"], mode="markers",
+    marker=dict(size=df["pop"], sizemode="area", sizeref=sizeref),
+))
```

## What the rule reports

The rule reports a go trace whose marker size is read from the data and whose size mode is left at the default or written as `"diameter"`. It also reports `sizemode="diameter"` given to `update_traces` in a file that calls a px function with `size=`. The first form covers `marker=dict(size=...)`, `marker={"size": ...}`, `marker=go.scatter.Marker(size=...)` and `marker_size=...` on the 11 trace types that have `marker.sizemode`: `go.Scatter`, `go.Scatter3d`, `go.Scattercarpet`, `go.Scattergeo`, `go.Scattergl`, `go.Scattermap`, `go.Scatterpolar`, `go.Scatterpolargl`, `go.Scattersmith`, `go.Scatterternary` and `go.Splom`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

unsafe for `sizemode="diameter"` on a px figure: drops the argument, so the markers get the area that px set. None for a go trace, whose `sizeref` depends on the data.

All 11 trace types resolved `marker.sizemode` to `"diameter"`, so a marker's area grows with the
square of its size. From 10 to 20 the data grows by 100 percent and the area by 300 percent, a
Lie Factor of 3.

| Size | Radius drawn with `"diameter"` | Area in square pixels with `"diameter"` | Area ratio with `"diameter"` | Area ratio with `sizemode="area"` and the `sizeref` above |
| --- | --- | --- | --- | --- |
| 10 | 5 | 78.5 | 1 | 1 |
| 20 | 10 | 314.2 | 4 | 2 |
| 40 | 20 | 1,256.6 | 16 | 4 |

In the `sizeref` formula, 40 is the diameter in pixels of the largest marker, and the formula is
the one on the plotly page about bubble charts. `px.scatter(size=...)` and the six other px
functions that take `size` set `sizemode="area"` themselves, so they are not reported.

The rule reads a size only when it takes a column by name, as `df["pop"]`, `df["pop"] / 10` and
`df.pop_total` do, from an object that the same trace reads `x`, `y` or another coordinate from.
`go.scatter.Marker(size=df.pop_total)` resolved to `sizemode="diameter"` and drew the same radii
as `dict(size=...)`. The rule leaves alone the sizes in the table.

| Size | Why the rule leaves it alone |
| --- | --- |
| a pandas property such as `df.size` or `df.index`, or a method that is called, such as `np.full(...)` | it is not a column |
| a number, a bare name, a written list, a column of another object, or a size chosen with `np.where` or a condition | it may be a design choice |
| a root, a power or a logarithm of a column | with `np.sqrt(df["pop"])` the areas for 100, 400 and 1,600 were 78.5, 314.2 and 1,256.6 square pixels, a Lie Factor of 1, and `sizemode="area"` would then understate |
| a column named as a size, such as `df["size"]` | it was made for the markers |

The rule is silent in a file that names `sizemode` in any other way than `sizemode="diameter"`,
because the mode can be set on the figure afterwards. After
`update_traces(marker_sizemode="diameter")`, a px bubble chart resolved to `sizemode="diameter"`
and drew the size column as the diameter, so the rule reports the `update_traces` line. The rule
has no fix, because the `sizeref` that keeps the markers visible depends on the data.
`sizemode="area"` alone drew size 10 with a radius of 2.24 pixels.
