With two overlaid y axes, where the lines cross depends on the two ranges you pick and not on the data. Draw the series in two panels that share x, or index both to a common start.

```diff lang="python"
-fig = make_subplots(specs=[[{"secondary_y": True}]])
-fig.add_trace(go.Scatter(x=months, y=sales, name="sales"), secondary_y=False)
-fig.add_trace(go.Scatter(x=months, y=visits, name="visits"), secondary_y=True)
+fig = make_subplots(rows=2, cols=1, shared_xaxes=True)
+fig.add_trace(go.Scatter(x=months, y=sales, name="sales"), row=1, col=1)
+fig.add_trace(go.Scatter(x=months, y=visits, name="visits"), row=2, col=1)
```

## What the rule reports

The rule reports a cell with `"secondary_y": True` in the `specs` of `make_subplots`, and an axis with `overlaying="y"` or `overlaying="x"` in `update_layout`, `go.Layout` or `go.Figure`.

The rule reports how the axes are set up, and it does not prove that traces use both axes. An
unused second axis draws no second scale, so the advice applies when traces use both overlaid
axes with independent ranges.

## How to fix it

xpd has no automatic fix, so make the change by hand.

"Show data variation, not design variation" (page 61). Two y axes in one plot have two free
scales, so the ranges, and not the data, decide where the lines cross and which
line looks steeper.
The figure above resolved the second axis to `overlaying="y"` on the right with no `matches`,
and the first trace got the range 8.6 to 31.4.

| Range of the second axis | Lines crossed at |
| --- | --- |
| 393 to 507, chosen by plotly | x=1.67 |
| 0 to 1000, set with `update_yaxes(range=[0, 1000], secondary_y=True)` | x=1.81 |

The rule reports the cell of `specs` and not each `add_trace`, so a figure gets one report for
each cell with two axes. It leaves alone an axis that sets `overlaying` together with `matches`,
because `matches` shares the range with the first axis. Both axes then resolved to -6 to 107. The underscore form
`yaxis2_overlaying="y"` raised `ValueError` in `update_layout` on a `go.Figure`, so the rule
does not read it.
