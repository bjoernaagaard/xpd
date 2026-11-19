An animation shows one frame at a time, so the reader compares frames from memory. Draw the frames side by side as small multiples with `facet_col`, or with `make_subplots`.

```diff lang="python"
-fig = px.scatter(gap, x="gdpPercap", y="lifeExp", animation_frame="year")
+fig = px.scatter(gap, x="gdpPercap", y="lifeExp", facet_col="year", facet_col_wrap=4)
```

## What the rule reports

The rule reports `animation_frame="column"` on the 28 px functions that take it. The message names `facet_col` for the 17 that have it, and `make_subplots` for the other 11. It also reports frames built with `go.Frame`, as `frames=` in `go.Figure` or in `update` on a plotly figure, and as `fig.frames = ...`.

The rule reports how the frames are set up. An empty figure or a single frame has no different
data to compare, so the advice to draw small multiples applies when several frames show
different data and their panels fit on the page.

The rule leaves alone frames copied from another figure, as `frames=fig.frames`, because the
figure they come from is reported where it is made. It also leaves alone `frames=None` and
`frames=[]`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

"Small multiples resemble the frames of a movie: a series of graphics, showing the same
combination of variables, indexed by changes in another variable" (page 170). The book lists
them as "inevitably comparative" (page 175). An animation shows one frame at a time, so the reader
compares the frames from memory.

| Figure | Frames or panels | Points drawn at once |
| --- | --- | --- |
| gapminder, `animation_frame="year"` | 12 frames | 142 of the 1,704 rows |
| gapminder, `facet_col="year", facet_col_wrap=4` | 12 panels | all 1,704 rows |
| `go.Figure` with three frames of one point each | 3 frames | 1 |
| the same three points in `make_subplots(rows=1, cols=3)` | 3 panels | 3 |

`full_figure_for_development` keeps no frames, so the frames were counted in the figure and in
the page that `to_html` wrote, which passed the three frames to `Plotly.addFrames`.

The rule is info and has no fix, because panels need room and an animation can be the better
choice when there are many frames. 11 of the 28 functions have no `facet_col`, and
`px.scatter_3d(facet_col=...)` raised TypeError, "got an unexpected keyword argument
'facet_col'". For the 11, which are `bar_polar`, `choropleth_map`, `density_map`, `line_3d`,
`line_map`, `line_polar`, `line_ternary`, `scatter_3d`, `scatter_map`, `scatter_polar` and
`scatter_ternary`, the message says to draw one panel for each frame with `make_subplots`.
