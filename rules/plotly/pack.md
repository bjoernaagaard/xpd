# plotly

The plotly rules report figures whose drawn sizes do not follow the numbers, ink that shows no
data, colors without an order, and chart forms and words that make a figure harder to read. They
apply Edward Tufte's book "The Visual Display of Quantitative Information", second edition, to
plotly figures. Each rule names the principle of the book that the figure breaks and what the
reader then sees, and it also names the plotly code to write instead. A page number on the plotly rule pages is a
printed page of the book.

| Group | What its rules report |
| --- | --- |
| Sizes that do not follow the numbers | marker areas, bar lengths, wedges, axes and scales that draw a number larger or smaller than it is |
| Ink that shows no data | tinted templates, grids, frames, boxes, repeated colors, hatching and gauges |
| Color without an order | rainbow color scales |
| Chart forms the book argues against | pies, sunbursts, splines, animations, smoothed heatmaps and charts of a few numbers |
| Words on the figure | titles in capitals, legends in place of labels, rotated tick labels and column names as axis titles |

The rules run on files that import `plotly`, and on files that make plotly the plotting backend
of pandas. Every figure measurement on the plotly rule pages used plotly 7.1.0 with pandas 3.1.1 on
Python 3.14. plotly.js resolved each figure through `full_figure_for_development`, which fills
in every default. Where a rule is about a drawn size, the figure was rendered to SVG and the
size was read from the drawn path. The examples assume `import plotly.express as px`,
`import plotly.graph_objects as go` and `import plotly.io as pio`, and names such as `df` and
`fig` stand for values from your program.

A rule reports a call only when the file binds the name in front of the call to a plotly module.

| Code | Proof that the name is plotly |
| --- | --- |
| `import plotly.express as px` or `from plotly import graph_objects as go`, under any alias | yes |
| the full dotted name, as in `plotly.express.pie(...)` | yes |
| a name `px` or `go` that the file binds to something else | no |

A file that makes plotly the plotting backend of pandas need not import plotly. xpd then reads
`df.plot.bar(...)` as `px.bar(df, ...)`, because on pandas 3.1.1 with plotly 7.1.0,
`df.plot.bar(x="team", y="score")` was equal, as JSON, to `px.bar(df, x="team", y="score")`.

```text
on(a file that holds pd.options.plotting.backend = "plotly" or pd.set_option("plotting.backend", "plotly"))
  if the file names no other backend, in an assignment, in set_option or as backend= of a call
    read df.plot.line, bar, area, scatter and box as the px function of the same name
      only when every argument of the call is named   # px.bar takes the frame first, df.plot.bar does not
not read
  df.plot(kind="bar")                     # plot-kind-argument of the pandas pack rewrites it to df.plot.bar
  a file where another module sets the backend
```

[`plot-kind-argument`](../pandas/plot-kind-argument.md) is the pandas rule that rewrites `df.plot(kind=...)`.

A rule about a method, such as `fig.update_xaxes(...)`, needs proof that the object is a plotly
figure. The proof holds when the method is called on the call that makes the figure, as in
`px.bar(...).update_xaxes(...)`, or on a variable that holds one. xpd follows the variable to
its value in three ways:

- The nearest earlier statement that binds the variable is an assignment from such a call,
  however many statements lie between the two. The statement is in the same block as the
  method call or in a block around it, up to the start of the function. A variable assigned
  from another variable, as in `chart = fig`, holds what the other variable held.
- The variable is a parameter with the annotation `go.Figure` or `go.FigureWidget`, or an
  assignment gives it that annotation, as in `fig: go.Figure = load()`.
- The variable is a parameter without that annotation, and every call of its function in the
  file passes a plotly figure. The function must be called at least once, by its name alone.
  A method, a function with a decorator, a function that is passed on as a value and a call
  with `*args` or `**kwargs` give no proof, because the file does not show every argument.

Any other binding of the name on the way ends the proof. Such a binding is a second assignment
that is not a figure, a loop variable, a `with ... as`, an import, a `def` or a `del` of the same
name, or an assignment inside an `if` or a loop that stands before the method call or around it.
A function that is defined in one module and called from another is not followed, so its
parameter needs the annotation.

A figure made by `go.Figure()` and filled later with `fig.add_trace(...)` proves a plotly figure,
and the statements after the assignment prove what it holds. For example, an `add_trace` of a
`go.Bar` makes it a bar chart for [`bar-axis-without-zero`](bar-axis-without-zero.md), when no
second assignment of the name comes between the two. The annotation `go.Figure` alone does not
prove a bar chart.

A rule reads a value only where it is written in the call, as a number, a string, `True` or
`None`. A name in the same place may hold any value, so the rule is silent.

No document of the pack carries `since`, because the names that the messages tell you to write
exist on plotly 5.0.0, 5.24.1, 6.0.0 and 7.1.0, where each was run.

## The proof each rule needs

Every rule has a measurement that was run, and its message quotes what was measured. A
measurement is one of two kinds:

- A resolved value is what plotly.js made of the figure, read from
  `full_figure_for_development`, such as `sizemode="diameter"` for a `go.Scatter` that sets no
  size mode.
- A drawn size is read from the SVG that plotly renders, such as the radius of a marker, the
  visible length of a bar, or the highest point of a curve.

| Severity | When |
| --- | --- |
| `error` | the figure is wrong for every reader |
| `warning` | the drawn sizes do not follow the numbers, or the book rejects the form outright |
| `info` | the form is a choice that the book argues against |

The rules were run over a corpus of 1,087 Python files, read as text and never imported or run.
The rule table after the next two paragraphs gives the reports that the current build makes on
the corpus.

| Part of the corpus | Files | What the files are |
| --- | --- | --- |
| docs | 151 | the `python` code blocks of each page of plotly's documentation |
| Dash | 117 | Python files of Dash that name plotly |
| public | 819 | public files that GitHub code search found for the import lines of plotly express, graph objects and `make_subplots` |

Recall, the share of planted mistakes that a rule finds, was measured on forms planted in copies
of 47 of the corpus files. The 1,212 planted forms are 48 in six groups for each rule and 60 in
eight groups for `free-facet-scales`. The groups hold settings written in the call, later updates, dictionaries, objects, and
values held in a name where the form allows one. A planted form counts as found when its rule
reports the lines of the form, and a report that was already in the file does not count. Every
copy passed `ast.parse`, and none was run.

The 47 files are not an unseen set, because they were seen while the rules were written.
Some of their reports had been read before. A planted form counts only
when the data has the values that the form assumes, such as a numeric column. Forms that no rule
reads stay in the count, such as a box given as a `**` dictionary, a bad axis title written out,
and a flat `go.Surface`. An `info` report is a design choice and does not prove that the figure
is wrong.

| Rule | Severity | Page | Measurement | Reports: docs, Dash, public | Total reports | Files with report | Recall | Fix |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [`marker-size-diameter`](marker-size-diameter.md) | error | 57 | drawn: areas 1, 4, 16 for sizes 10, 20, 40 | 1, 0, 6 | 7 | 6 | 39 of 48 | none |
| [`bar-axis-without-zero`](bar-axis-without-zero.md) | warning | 57 | drawn: bars of 95 and 100 at 163.6 and 327.3 pixels, 145.4 and 290.9 in a `go.Figure` | 0, 0, 0 | 0 | 0 | 37 of 48 | none |
| [`area-axis-without-zero`](area-axis-without-zero.md) | warning | 57 | drawn: fills of 95 and 100 at 163.6 and 327.3 pixels | 0, 0, 0 | 0 | 0 | 37 of 48 | none |
| [`polar-bar-area`](polar-bar-area.md) | warning | 57 | drawn: radii 1, 2, 3, 4 for values 1, 2, 3, 4 | 10, 4, 1 | 15 | 8 | 48 of 48 | none |
| [`three-d-for-two-d`](three-d-for-two-d.md) | error, warning | 71 | resolved: `visible=False`. Resolved: z axis from -1 to 1 | 0, 0, 0 | 0 | 0 | 32 of 48 | none |
| [`dual-y-axis`](dual-y-axis.md) | info | 61 | resolved: lines cross at 1.67 or at 1.81 | 15, 5, 41 | 61 | 36 | 32 of 48 | none |
| [`free-facet-scales`](free-facet-scales.md) | info | 170 | resolved: one value at 0.70 and at 0.45 of the panel. Drawn: bars of 5 and 10 equally tall | 2, 0, 1 | 3 | 2 | 40 of 60 | none |
| [`default-template`](default-template.md) | info | 96 | resolved: `#E5ECF6`. Drawn: 10 grid lines | 102, 18, 231 | 351 | 351 | 32 of 48 | unsafe |
| [`explicit-grid`](explicit-grid.md) | info | 112 | drawn: 0 grid lines became 10 | 9, 4, 28 | 41 | 19 | 40 of 48 | unsafe |
| [`boxed-axis-frame`](boxed-axis-frame.md) | info | 103 | drawn: 1 axis line segment became 2 | 7, 1, 18 | 26 | 7 | 40 of 48 | none |
| [`box-plot-ink`](box-plot-ink.md) | info | 124 | drawn: 36 box segments and 16 offset quartile segments for four groups | 5, 0, 14 | 19 | 9 | 40 of 48 | none |
| [`redundant-color`](redundant-color.md) | info | 100 | drawn: 4 legend entries equal to the 4 tick labels. Drawn: 3 shades for 3 bar lengths | 16, 0, 70 | 86 | 55 | 40 of 48 | none |
| [`pattern-fill`](pattern-fill.md) | warning | 108 | drawn: 2 pattern images where there were 0 | 19, 0, 1 | 20 | 11 | 40 of 48 | none |
| [`gauge-indicator`](gauge-indicator.md) | info | 168 | drawn: 8 shapes and 7 labels for one number | 4, 2, 30 | 36 | 25 | 40 of 48 | none |
| [`rainbow-colorscale`](rainbow-colorscale.md) | warning | 154 | resolved: Jet lightness 13.5, 97.1, 25.5 along the scale | 1, 0, 7 | 8 | 6 | 40 of 48 | none |
| [`pie-chart`](pie-chart.md) | warning | 178 | resolved: trace type `pie`, 2 traces with a facet or in 2 cells | 28, 7, 120 | 155 | 84 | 48 of 48 | none |
| [`sunburst-as-pie`](sunburst-as-pie.md) | warning | 178 | drawn: a wedge of 45 with 2.25 times the area of one of 60 | 14, 2, 11 | 27 | 15 | 48 of 48 | none |
| [`spline-line`](spline-line.md) | info | 92 | drawn: 10.55 where the data ends at 10 | 4, 0, 7 | 11 | 8 | 40 of 48 | unsafe |
| [`animation-frame`](animation-frame.md) | info | 170 | resolved: 142 of 1,704 rows at once. Drawn: 1 of 3 frames | 11, 1, 27 | 39 | 22 | 40 of 48 | none |
| [`heatmap-smoothing`](heatmap-smoothing.md) | info | 92 | drawn: 252 colors for a grid of two values | 0, 0, 0 | 0 | 0 | 40 of 48 | unsafe |
| [`table-for-small-data`](table-for-small-data.md) | info | 56 | drawn: 0 of 4 values written, 4 of 4 in `go.Table` | 31, 0, 1 | 32 | 12 | 29 of 48 | none |
| [`legend-instead-of-labels`](legend-instead-of-labels.md) | info | 180 | drawn: 2 legend entries became 0 with labels | 7, 3, 71 | 81 | 59 | 21 of 48 | none |
| [`rotated-tick-labels`](rotated-tick-labels.md) | info | 183 | drawn: 2 of 2 category labels rotated | 4, 4, 71 | 79 | 51 | 40 of 48 | none |
| [`all-caps-title`](all-caps-title.md) | info | 183 | drawn: a title 1.31 times as wide in capitals | 3, 0, 1 | 4 | 2 | 40 of 48 | none |
| [`axis-title-column-name`](axis-title-column-name.md) | info | 183 | resolved: axis title `gdpPercap` | 68, 1, 104 | 173 | 71 | 15 of 48 | none |

The corpus counts say how often a rule reports, and not its recall or whether the advice suits
a given chart. `area-axis-without-zero`, `bar-axis-without-zero`, `heatmap-smoothing` and
`three-d-for-two-d` made no report on the corpus.

Every rule but `box-plot-ink`, `pie-chart`, `polar-bar-area` and `sunburst-as-pie` missed the 8
planted forms that hold the value in a name. The table gives every other planted form that a
rule missed.

| Rule | Other missed forms |
| --- | --- |
| `marker-size-diameter` | a size in a `go.Scatter` marker dictionary (1) |
| `bar-axis-without-zero` | `update_layout(yaxis_range=...)` (2), an axis dictionary on an added `go.Bar` (1) |
| `area-axis-without-zero` | `update_layout(yaxis_range=...)` (2), a filled `go.Scatter` (1) |
| `three-d-for-two-d` | a flat `go.Surface` (8) |
| `dual-y-axis` | `update_yaxes(overlaying=...)` (8) |
| `free-facet-scales` | `update_layout(yaxis_matches=...)` (8), a loop that adds a `go.Scatter` to subplots (3), a loop that adds a `go.Bar` to subplots (1) |
| `default-template` | a template set with `Figure.update_layout` (8) |
| `box-plot-ink` | a box given as a `**` dictionary and a trace name (8). It found the 8 forms that hold the values in a name. |
| `table-for-small-data` | written values in `px.pie` (3), a bare `go.Bar` with `y` (3), written values in `px.bar` (2), a `go.Figure` with a `go.Pie` list (2), a bare horizontal `go.Bar` with `x` (1) |
| `legend-instead-of-labels` | `px.line` with `color` (5), `px.line` with a list of columns (5), `px.line` with `color` and markers (5), named `go.Scatter` traces added one by one (2), a `go.Figure` with a list of named `go.Scatter` traces (2) |
| `axis-title-column-name` | an axis title written out (8), a column in `px.scatter` (5), `px.bar` (5), `px.area` (4) and `px.line` (3) |

No fix of the pack is safe, because every fix changes what the reader sees. `--fix` alone writes
nothing for the pack, and `--fix --unsafe-fixes` writes four rewrites:

```diff
-template="ggplot2"          # or "plotly", "seaborn"
+template="simple_white"
-showgrid=True
+showgrid=False
-line_shape="spline"
+line_shape="linear"
-zsmooth="best"              # or "fast"
+zsmooth=False
```

## What the pack does not check

The book states principles that the code of a figure does not show, so no rule covers them.

| Principle | Page | What the code does not show |
| --- | --- | --- |
| "In time-series displays of money, deflated and standardized units of monetary measurement are nearly always better than nominal units" | 68 | whether a column is money |
| "Graphics must not quote data out of context" | 74 | the rows that were left out |
| The range-frame and the dot-dash-plot | 130, 133 | plotly has no axis line that ends at the lowest and highest value, and xpd cannot ask for `marginal_x="rug"` from a missing argument |
| The Shrink Principle | 169 | how many values the figure shows, which sets its right size |

## Sizes that do not follow the numbers

The rules in the group report a figure that draws a number larger or smaller than it is. The
first principle of graphical integrity in the book is "The representation of numbers, as
physically measured on the surface of the graphic itself, should be directly proportional to
the numerical quantities represented" (page 56). The book measures a break of the principle
with the Lie Factor, and calls a factor above 1.05 or below 0.95 a substantial distortion
(page 57).

```text
Lie Factor = size of the effect shown in the graphic / size of the effect in the data
```

- marker-size-diameter
- bar-axis-without-zero
- area-axis-without-zero
- polar-bar-area
- three-d-for-two-d
- dual-y-axis
- free-facet-scales

## Ink that shows no data

The rules in the group report ink that a figure can lose without losing data. The book defines
the data-ink ratio as the share of a graphic's ink that shows data, and asks to "erase
non-data-ink, within reason" (page 96) and to "erase redundant data-ink, within reason"
(page 100).

- default-template
- explicit-grid
- boxed-axis-frame
- box-plot-ink
- redundant-color
- pattern-fill
- gauge-indicator

## Color without an order

The rule in the group reports a color scale whose colors do not get lighter or darker in one
direction. The book says that "the mind's eye does not readily give a visual ordering to colors,
except possibly for red to reflect higher levels than other colors", and that "varying shades
of gray show varying quantities better than color" (page 154).

- rainbow-colorscale

## Chart forms the book argues against

The rules in the group report pies, sunbursts, smoothed lines and cells, animations, and charts
of a few numbers that a table shows better.

- pie-chart
- sunburst-as-pie
- spline-line
- animation-frame
- heatmap-smoothing
- table-for-small-data

## Words on the figure

The rules in the group report words that are harder to read than they need to be. The book lists
what makes the words of a graphic friendly or unfriendly (page 183), and each rule quotes one
line of the list.

- all-caps-title
- legend-instead-of-labels
- rotated-tick-labels
- axis-title-column-name
