`zsmooth` blends the colors of neighboring cells, so the heatmap shows values the data does not hold. Pass `zsmooth=False`.

```diff lang="python"
-fig = go.Figure(go.Heatmap(z=grid, zsmooth="best"))
+fig = go.Figure(go.Heatmap(z=grid, zsmooth=False))
```

## What the rule reports

The rule reports a `zsmooth` value that blends the colors of the cells.

| Call | Values reported |
| --- | --- |
| `go.Heatmap` | `"best"`, `"fast"` |
| `go.Histogram2d`, `update_traces` on a plotly figure | `"best"` |
| `go.Image` that sets `source` | `"fast"` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `False`.

"Above all else show the data" (page 92). Smoothing fills the space between the cells with
colors that blend their values.

| A 2 by 2 grid of 0 and 10 in `"Greys"`, 400 by 400 pixels | Colors drawn |
| --- | --- |
| `go.Heatmap(zsmooth=False)` | 2 |
| `go.Heatmap(zsmooth="fast")` | 61 |
| `go.Heatmap(zsmooth="best")` | 252 |
| `go.Histogram2d(zsmooth="best")` | 252 |
| `go.Histogram2d(zsmooth="fast")`, which resolved to `False` | 2 |
| `go.Image(zsmooth="fast")`, a checkerboard given as `z` | 2 |
| `go.Image(zsmooth="fast")`, the same pixels given as a PNG in `source` | 61 |

The two colors of the data covered all 160,000 pixels without smoothing, and 45,156 and 40,401
pixels with it.

The rule reads `"fast"` on `go.Heatmap` only, and on `go.Image` only with `source`, because the
docstring of `go.Image` says that `zsmooth` "only applies for image traces that use the source
attribute". `go.Image` takes `"fast"` or `False`, and `"best"` raised ValueError.
`update_traces(zsmooth="fast")` is left alone, because the rule cannot see which traces the
figure holds.

The fix is unsafe, because the figure looks different when each cell is drawn as a block of one
color.
