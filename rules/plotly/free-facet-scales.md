`matches=None` gives each facet its own scale, so the same value sits at a different height in each panel. Keep one scale across the facets.

```diff lang="python"
 fig = px.scatter(tips, x="total_bill", y="tip", facet_col="sex")
-fig.update_yaxes(matches=None)
```

## What the rule reports

The rule reports `update_yaxes(matches=None)` and `update_xaxes(matches=None)` on a px figure that was made with `facet_col`, `facet_row` or `facet_col_wrap`. It also reports a loop that adds a `go.Scatter`, `go.Scattergl`, `go.Bar`, `go.Box` or `go.Violin` with the same written column as `y` to the cells of a `make_subplots` figure without `shared_yaxes`, where the row or the col changes with the loop.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

unsafe for `matches=None`: drops the argument, so the panels share one scale again, and drops comments inside the parentheses. None for `make_subplots`, where `shared_yaxes` goes to another statement.

The book says of small multiples, a grid of small charts drawn the same way, "The design remains
constant through all the frames, so that attention is devoted entirely to shifts in the data" (page 170). px draws facets on one scale,
and `matches=None` frees the scales. A reader who compares heights across the panels then reads
a difference that the data does not have. Free scales are right when the panels hold different
units, which is why the rule is info.

| Facets of `px.scatter` | Second y axis | Ranges of the two panels | A tip of 5 drawn at |
| --- | --- | --- | --- |
| as px draws them | `matches="y"` | 0.38 to 10.62 in both | the same height in both |
| after `update_yaxes(matches=None)` | no `matches` | 0.62 to 6.88, and 0.38 to 10.62 | 0.70 of the height of one panel, and 0.45 of the other |

`make_subplots` gives every cell its own y axis. The table gives a bar of 5 in one cell and a bar
of 10 in the other.

| `make_subplots` | Ranges | Bar heights in pixels |
| --- | --- | --- |
| `rows=1, cols=2` | 0 to 5.26, and 0 to 10.53, with no `matches` | 304.0 and 304.0 |
| `rows=1, cols=2, shared_yaxes="all"` | 0 to 10.53 in both | 152.0 and 304.0 |
| `rows=2, cols=1, shared_yaxes=True` | 0 to 5.26, and 0 to 10.53 | not measured |

`shared_yaxes=True` shares the axes of one row only, so the message names `"all"`.

For example, the rule reports this loop.

```python
fig = make_subplots(rows=1, cols=2)
for i, (shop, part) in enumerate(df.groupby("shop")):
    fig.add_trace(go.Bar(x=part["month"], y=part["sales"]), row=1, col=i + 1)
```

Share one scale instead.

```python
fig = make_subplots(rows=1, cols=2, shared_yaxes="all")
```

The rule reads a loop because a column written as `y=part["sales"]` in each pass proves that
every panel holds the same measure. It leaves alone a column named `value` or `values`, because
`pd.melt` gives that name to the column of a long table, which holds a different measure in
each panel. It also leaves alone a trace added with `secondary_y`, and a file that names
`matches`, `shared_yaxes` or a y axis range anywhere, because the file may set the scales
afterwards.
