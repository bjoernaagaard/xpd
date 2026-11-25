---
title: Review a chart against Tufte's book
description: Ask the questions of "The Visual Display of Quantitative Information" for each chart of a file, in plotly or any other library, and measure the drawn figure.
sidebar:
  order: 6
---

The plotly rules apply Edward Tufte's book "The Visual Display of Quantitative Information",
second edition, to plotly code. Much of the book is about what the code of a figure does not
show, such as whether money is deflated or whether a change is shown with the years around it.
`xpd chart-review` covers the whole book for each chart of a file.

| It prints | From |
| --- | --- |
| Each chart call of the file, with its kind and the plotly call that draws it | A call table for plotly express, plotly graph objects, Dash, matplotlib, seaborn and pandas |
| The plotly rules that fire on the file | [`xpd check`](../../commands/check/) with the plotly pack |
| Each question of the book that applies to a chart, with its page, what to look at and the plotly code to write | `rules/xpd/chart-review.yml`, which holds 46 questions |
| With `--run <adapter.py>`, the measured facts of the drawn figure under the questions they answer | `figures()` of the adapter, drawn in the project's Python |

A page number is a printed page of the second edition. The rule pages of the
[plotly pack](../../rules/plotly/) show the questions that each rule reports in code.

## How an agent uses it

With the [xpd plugin](../claude-code/), a prompt about a chart gets one line from the hook, in a
project of any language. The agent then runs the review on each file that draws the chart.

```console
$ echo '{"cwd":"/tmp/project","hook_event_name":"UserPromptSubmit","prompt":"review the charts in my dashboard"}' | xpd hook
Chart task: run `xpd chart-review <file>` on each file that draws the chart. It lists the plotly rules that fire and the questions from Tufte's book for each chart, in plotly or any other library, with the plotly route. Add `--run <adapter.py>` to measure the drawn figure. For a new chart, `xpd api xpd/recipe/plotly-chart` prints the plotly-chart playbook, which names the same command.
```

1. Run `xpd chart-review <file>`.
2. Fix each plotly rule that fires.
3. Answer each question from the chart and its data, and change the chart where the answer is no.
4. Redraw a chart of another library with its plotly route, unless the user keeps the library.
5. Add `--run <adapter.py>` when the app can run, so the measured facts answer the questions
   that the code cannot.
6. Run the review again. The chart is done when no plotly rule fires and each question has an
   answer.

## A plotly chart

The file below draws a pie of four regions with its title in capitals, and twenty years of a
price in nominal euro.

```python title="sales.py"
import plotly.express as px


def sales_pie():
    return px.pie(
        names=["North", "South", "East", "West"],
        values=[40, 30, 20, 10],
        title="SALES BY REGION",
    )


def price_line(years, prices):
    return px.line(x=years, y=prices, labels={"x": "Year", "y": "Price in euro"})
```

The review names the pie and the capitals by the rules that fire, and asks about the nominal
euro under the principle of deflated money. The `...` lines stand for the questions left out
here.

```console
$ xpd chart-review sales.py --format text
Chart review of sales.py
Libraries: plotly
Charts:
  line 5: px.pie draws a pie chart in plotly. Plotly route: go.Table, or px.bar sorted by value
  line 13: px.line draws a line chart in plotly. Plotly route: px.line
Plotly rules that fire:
  sales.py:1:1 info [plotly/default-template] This file sets no template, and plotly then draws with its default "plotly", which fills the plot area with #E5ECF6 and drew 10 white grid lines on a scatter plot. That ink shows no data, and Tufte asks to erase non-data-ink. Set pio.templates.default = "simple_white" once, which drew a white plot area with axis lines and no grid, or pass template="simple_white" to each figure.
  sales.py:5:12 warning [plotly/pie-chart] A pie chart puts no numbers along a common scale, so the reader compares angles. Tufte writes that pie charts should never be used and that a table is nearly always better. Show the shares as a sorted px.bar, or as a go.Table when they are few.
  sales.py:5:12 info [plotly/table-for-small-data] This pie shows [40, 30, 20, 10], a few numbers written in the code, as angles and writes them as percentages at most. Four values drawn with go.Pie wrote their shares and none of the values, where go.Table wrote all four. Tufte writes that tables usually beat graphics for small data sets of 20 numbers or less. Show them with go.Table(cells=dict(values=[names, values])).
  sales.py:8:15 info [plotly/all-caps-title] The title "SALES BY REGION" is set in capitals only. Drawn at 20 pixels, "SALES BY REGION" was 1.31 times as wide as "Sales by region", and words in capitals all have the same rectangular outline, so they are slower to read. Tufte lists upper and lower case type as friendly and all capitals as unfriendly. Write the title in sentence case, and keep capitals for abbreviations.
Questions from Edward Tufte, "The Visual Display of Quantitative Information", second edition, 39 of 46:
...
deflate-money (page 68): Deflated money
  If the chart shows money over time, is it in deflated and standardized units, such as constant euro per person, and does a label say whether it is nominal or real?
  Look at: A currency in the axis title or data, the years plotted, and the word nominal, real or constant with a base year.
  In plotly: Divide by a price index (and population) before px.line and title the axis like "Price, 2015 euro".
  Charts at lines: 13
...
pie (page 178): Pie charts
  Is this a pie? A table, or bars sorted along a common scale, shows the numbers better, and several pies are worse still.
  Look at: Pie, donut and sunburst marks and how many slices they hold.
  In plotly: Use go.Table, or px.bar sorted by value.
  Charts at lines: 5
  Rules: plotly/pie-chart (fires), plotly/sunburst-as-pie

...
```

## A chart in another library

A chart in matplotlib, seaborn or pandas gets the same questions, and each chart gets the plotly
call that draws it, because plotly is the house library. No plotly rule reads its code. The file
below draws one tall panel with two lines that only red and green tell apart, for the year before
and the year after a policy.

```python title="prices.py"
import matplotlib.pyplot as plt


def price_chart():
    fig, ax = plt.subplots(figsize=(4, 9))
    ax.plot([2019, 2021], [20.0, 35.0], color="red", label="North")
    ax.plot([2019, 2021], [18.0, 30.0], color="green", label="South")
    ax.set_title("PRICE BEFORE AND AFTER THE 2020 POLICY")
    ax.set_ylabel("Price in euro")
    ax.legend()
    return fig
```

The code shows none of the faults, so the run measures them. The adapter is a Python file with
`figures()`, which returns a dict of name to figure.

```python title="figures.py"
from prices import price_chart


def figures():
    return {"prices": price_chart()}
```

```console
$ xpd chart-review prices.py --run figures.py --format text
Chart review of prices.py
Libraries: matplotlib
Charts:
  line 6: ax.plot draws a line chart in matplotlib. Plotly route: px.line
  line 7: ax.plot draws a line chart in matplotlib. Plotly route: px.line
The plotly rules do not read matplotlib. The questions below apply to every library, and each chart's plotly route is the call that draws it in plotly, the house library.
Plotly rules: none fires.
The run of figures.py measured 1 figure: prices
Questions from Edward Tufte, "The Visual Display of Quantitative Information", second edition, 37 of 46:

...
deflate-money (page 68): Deflated money
  If the chart shows money over time, is it in deflated and standardized units, such as constant euro per person, and does a label say whether it is nominal or real?
  Look at: A currency in the axis title or data, the years plotted, and the word nominal, real or constant with a base year.
  In plotly: Divide by a price index (and population) before px.line and title the axis like "Price, 2015 euro".
  Charts at lines: 6, 7
  Measured in prices (x-values): North 2 x values, years 2019 to 2021; South 2 x values, years 2019 to 2021. North has only two x values, 2019 and 2021, with nothing before or after them; South has only two x values, 2019 and 2021, with nothing before or after them
  Measured in prices (text): title 'PRICE BEFORE AND AFTER THE 2020 POLICY', axis titles 'Price in euro'. the title 'PRICE BEFORE AND AFTER THE 2020 POLICY' is written in capitals; money (PRICE) with no word that says nominal, real or constant

...
context (page 74): Context is essential
  Does the chart show enough data around its point, the years before and after a change and comparison series, to answer compared with what?
  Look at: The number of time points, whether the change is shown against the trend before it, and comparison groups.
  In plotly: Plot all years with px.line and add the comparison series in the same figure.
  Charts at lines: 6, 7
  Measured in prices (x-values): North 2 x values, years 2019 to 2021; South 2 x values, years 2019 to 2021. North has only two x values, 2019 and 2021, with nothing before or after them; South has only two x values, 2019 and 2021, with nothing before or after them
...
color-deficient (page 183): Color-deficient readers
  Can a reader who cannot tell red from green still tell the series apart, through labels, line style, position or a blue contrast?
  Look at: Series told apart only by red and green, and colors that carry an essential contrast.
  In plotly: Label lines directly and use line_dash or a blue against orange pair.
  Charts at lines: 6, 7
  Measured in prices (colors): North red -, South green -; no direct labels, a legend. North and South are told apart only by red against green, with the same line style and no direct labels
...
horizontal-shape (page 186): Proportion and shape
  Is the chart wider than tall, about half again as wide as tall when the data do not suggest another shape, and wider still for a wiggly curve?
  Look at: Width and height of the canvas and the number of panels.
  In plotly: Set fig.update_layout(width=900, height=600) or a 3 to 2 container.
  Charts at lines: 6, 7
  Measured in prices (canvas): 400 by 900 pixels, 1 panel. one panel 400 by 900 pixels is taller than wide
...
```

## Measure the drawn figure

`--run <adapter.py>` runs the adapter in the project's Python, the one that
[`xpd profile`](../../commands/profile/) uses, so it runs project code. It measures five facts of
each figure. A plotly figure is drawn by `full_figure_for_development`, which needs Kaleido and
Chrome. Without them, the facts come from the figure as written, and the review says so. A
matplotlib or seaborn figure is drawn with the Agg backend, which needs no browser. A run that
fails keeps every question and says why.

| Fact | What it measures | What it names |
| --- | --- | --- |
| `canvas` | Width and height in pixels, and the panels | One panel taller than wide |
| `values` | The values drawn by each trace, in all, and per square inch | 20 values or fewer, and the slices of a pie |
| `colors` | The color, dash and markers of each trace, the legend and the direct labels | Two series that only red and green tell apart |
| `x-values` | The x values of each trace and the years they span | A trace of two x values with nothing around them |
| `text` | The title, the axis titles, the tick angles and the annotations | A title in capitals, rotated tick labels, and money with no word for nominal, real or constant |

For a Dash app, the adapter imports the app and calls each callback with the inputs of the state
to review, so the figure is the one that the callback returns.

```python
import app


def figures():
    return {"revenue": app.revenue_chart("all regions", "2024")}
```

## The questions

`xpd chart-review --questions` prints all 46 questions and the five facts. Each question has an
`id`, the `page` and `line` of the book, the chart `kinds` it applies to (`any` for every chart),
the `facts` that answer it, the plotly code to write and the plotly `rules` that report it in
code. These are all of them, with their pages.

```console
$ xpd chart-review --questions --format json | jq -r '.questions[] | "page \(.page), \(.id), \(.principle): \(.text)"'
page 13, show-the-data, Show the data: Does the chart show the observations themselves, so a reader sees what a summary statistic or a fitted line would hide?
page 13, substance, Substance over method: Does the design send the reader to the substance of the data rather than to the method, the graphic design or the software, and would someone who knows the subject and the statistics stand behind it?
page 13, compare, Encourage comparison: Are the values the reader must compare placed side by side in one view, with marks of the same shape on a common scale?
page 13, overview-and-detail, Several levels of detail: Can a reader see the overall pattern at a glance and the fine structure up close, in one coherent display of a complex data set?
page 13, purpose, A clear purpose: What is the chart for, description, exploration, tabulation or decoration, and is a chart the right format for it, or would a sentence, a table or a combination serve better?
page 13, integrate-text, Graphics integrated with text: Is the chart run into the text that discusses it, in the same typeface, without ruled lines that wall it off, repeated near later references, and with Figure spelled out?
page 20, map-area, Area on maps: On a shaded map, does land area stand for importance where the quantity is people or counts, and could collection practice explain the pattern?
page 33, small-data-table, Tables for small data sets: Does the chart show 20 or fewer numbers, exact values, or many labeled values that a table or a sentence shows better?
page 51, multivariate, Graphical excellence is nearly always multivariate: Does the chart show the variables that explain the pattern, a scatter for a relationship between two variables and added explanatory or spatial variables for a time series, rather than one variable alone?
page 54, zero-baseline, Bars start at zero: Do the bars and filled areas start at zero, so a negative value or a difference is not disguised, and is a removed zero shown as removed?
page 56, labels, Clear, detailed labeling: Are the scales, units, sources and important events labeled on the chart, so no reading is ambiguous, and does an exploratory chart say how to read it?
page 57, lie-factor, The Lie Factor: Is the size of each mark proportional to its number? Divide the size of an effect in the chart by its size in the data; a Lie Factor outside 0.95 to 1.05 is a distortion.
page 61, design-variation, Show data variation, not design variation: Do scales, time intervals, periods and design stay constant across the chart, so every visible change is a change in the data?
page 68, deflate-money, Deflated money: If the chart shows money over time, is it in deflated and standardized units, such as constant euro per person, and does a label say whether it is nominal or real?
page 71, dimensions, No more dimensions than the data: Does the chart depict more varying dimensions than the data have, through 3D, perspective, areas or volumes for one-dimensional numbers?
page 74, context, Context is essential: Does the chart show enough data around its point, the years before and after a change and comparison series, to answer compared with what?
page 80, right-numbers, The right numbers: Are these the right numbers for the question: a sound model, enough data, and a statistic worth showing, rather than a thin or simple series dressed up?
page 96, data-ink, Maximize the data-ink ratio: Which ink shows no data? Could the background tint, frame, ticks, debris or uniform notes be erased or moved to the caption without loss, after a revision that compares versions?
page 98, repeat-cycle, Repeat a cycle: For a cyclical series such as hours, seasons or longitude, does the chart repeat part of the cycle or overlay its phases, so the eye can follow across the seam?
page 100, redundant-ink, Erase redundant data-ink: Does any number appear more than once, as a bar with its label and its own color, or as two symmetric halves, without a reason such as context or easier comparison?
page 108, vibration, Moire vibration: Do hatching, stripes, patterned fills, fields of ticks or doubled lines shimmer? Use flat gray tints or labels in words.
page 112, grid, Mute the grid: Is the grid muted or gone, fainter than the data, kept only where the chart is used to look up values, and drawn at meaningful values such as events?
page 116, duck, The duck: Has decoration taken over the chart, so that its structure is a style rather than data?
page 124, quartile-plot, The quartile plot: Would an offset quartile plot, a line with a gap for the middle half, show the five numbers with less ink than the box?
page 127, bar-erase, Redesign the bar chart: Could the bar chart drop its box, its vertical axis line and its ticks, with white grid lines cut through the bars and a thin baseline?
page 130, range-frame, The range-frame: Could the axis lines end at the data's minimum and maximum, labeled with those values, so the frame itself reports the range?
page 133, marginal, The dot-dash-plot: Would the marginal distribution of each variable, as a rug or a quartile plot on the frame, show what the scatter hides?
page 139, multifunction, Multifunctioning graphical elements: Could a label, a tick, a grid line or an identifier also carry data, such as exact values as ticks, grid lines at events, identifiers in a meaningful order, or digits as marks?
page 139, puzzle, Graphical puzzles: Can the reader see the pattern directly, without translating color codes, symbols or abbreviations into words through a key?
page 145, zero-and-missing, Zero and missing: Are zero values and missing values drawn differently, so an absent bar or a blank cell is not read as zero?
page 154, color-use, Color with a natural order: Does color show quantity with an ordered lightness scale and a natural metaphor, in light or transparent tones that keep labels readable?
page 155, lines-of-sight, Lines of sight: Is the display ordered so the eye reads it in a clear hierarchy, with steady horizontal or vertical lines of sight and aligned values?
page 168, many-numbers, Data density and the shrink principle: How many numbers does the chart show for its area, and could it show more data, or the same data in less space, while it stays legible?
page 169, scale-choice, Scales for time series: Is the vertical scale chosen for what the curve must show, neither so steep nor so flat that change is hidden, with zero removed only where the reader sees it removed and a log scale for relative change?
page 170, small-multiples, Small multiples: For a series of charts indexed by time or a category, are the panels side by side with the same design and the same scales, in a meaningful order?
page 177, craft, Careful production: Is the chart drawn with care in its technical details: no repeated or mistyped scale values, no clipped or overlapping text, one typeface?
page 178, order, Order by content: Are categories ordered by content or by value rather than alphabetically?
page 178, pie, Pie charts: Is this a pie? A table, or bars sorted along a common scale, shows the numbers better, and several pies are worse still.
page 183, words-spelled-out, Words spelled out: Are words spelled out on the chart, with no column names, codes or abbreviations?
page 183, horizontal-type, Words run left to right: Do all words, tick labels and the y axis title read from left to right?
page 183, labels-on-graphic, Labels on the graphic: Are series and areas labeled on the chart itself, so no legend is needed?
page 183, color-deficient, Color-deficient readers: Can a reader who cannot tell red from green still tell the series apart, through labels, line style, position or a blue contrast?
page 183, type-case, Type: Is the type clear, precise and modest, small for short labels, and in upper and lower case rather than capitals?
page 185, lines, Thin lines: Are lines thin, thinner as the data get dense, with the data line the heaviest where lines meet, and would calm shading under a series show its departures better?
page 186, horizontal-shape, Proportion and shape: Is the chart wider than tall, about half again as wide as tall when the data do not suggest another shape, and wider still for a wiggly curve?
page 191, judgment, Judgment: Where the chart breaks one of these principles, does the exception show the data better?
```

The JSON of a review has the keys `file`, `libraries`, `charts`, `issues`, `questions` and
`facts`. A question of a review also has `charts`, the lines of the charts it applies to,
`reported`, the rules of it that fire, and `measured`, the facts of `--run` that answer it.
