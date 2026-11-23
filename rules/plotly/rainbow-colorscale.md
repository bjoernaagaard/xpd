A rainbow colorscale such as Jet rises and falls in lightness, so it makes edges in the data that are not there. Use a scale whose lightness changes in one direction, such as Viridis.

```diff lang="python"
-fig = px.imshow(grid, color_continuous_scale="jet")
-fig = go.Figure(go.Heatmap(z=grid, colorscale=px.colors.sequential.Turbo))
+fig = px.imshow(grid, color_continuous_scale="Greys")
+fig = go.Figure(go.Heatmap(z=grid, colorscale="Viridis"))
```

## What the rule reports

The rule reports the color scales Jet, Rainbow, Turbo and HSV, written as a string in upper or lower case with or without `_r`, or as an attribute of `px.colors` such as `px.colors.sequential.Jet`, when it is given to `colorscale`, `color_continuous_scale` or a setting that ends in `_colorscale` inside a plotly call, or assigned to `px.defaults.color_continuous_scale`.

The rule reports a scale where the code sets it, also on a color axis that no trace uses. The
advice applies only when a trace colors numbers with the scale. A scatter plot without numeric
colors, or without a reference to the color axis, does not show the scale.

## How to fix it

`xpd check --fix --unsafe-fixes` replaces Jet, Rainbow, Turbo, and HSV strings and `px.colors` attributes with `"Viridis"` (or `"Viridis_r"` / `px.colors.sequential.Viridis`). It is unsafe because the best monotonic or diverging scale depends on your data.

The test of order is lightness. Each of the 94 named scales was resolved on a `go.Heatmap` and
sampled at 21 even steps, and the CIELAB lightness of each step was computed, from 0 for black
to 100 for white. A scale that changes direction draws a low and a high value equally dark.

| Scale | Changes of direction in lightness | Reported |
| --- | --- | --- |
| 61 scales, such as `"Greys"`, `"Blues"` and `"Viridis"` | 0 | no |
| Jet, which rose from 13.5 to 97.1 and fell back to 25.5 | 3 | yes |
| HSV | 3 | yes |
| Rainbow | 2 | yes |
| Turbo, with its lightest color in the middle | 1 | yes |
| Blackbody, Bluered, Portland, Picnic | 1, near one end or at a midpoint as the two-sided RdBu does | no |
| Earth, mrybm, mygbm | more than 0 | no, because they appear in no file the rules were run on |

`px.imshow` with `"jet"`, a `go.Scatter` marker with `"Jet"` and
`update_layout(coloraxis_colorscale="Jet")` all resolved to the same six stops.
`color_discrete_sequence` is left alone, because it colors groups and not quantities. Numbers
that name groups, such as cluster numbers, are better colored as categories, and the message
says so.
