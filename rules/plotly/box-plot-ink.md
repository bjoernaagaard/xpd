A box plot of one variable draws outlines that a five-number summary does not need. If only the summary matters, draw Tufte's quartile plot with a few lines and dots.

## What the rule reports

The rule reports `px.box` and `go.Box` with exactly one of `x` or `y`, when the call writes no point
settings, notches, means, quartile method, precomputed statistics, box style or orientation, and
has no `**` argument. A px call with `color`, a facet or an animation is left alone. The rule
does not prove that a box plot is wrong, and the advice holds only when the five numbers are
all the reader needs.

For example, the rule reports this box plot.

```python
fig = go.Figure(go.Box(y=values))
```

When only the minimum, quartiles, median and maximum are needed, draw this instead.

```python
low, q1, median, q3, high = 1, 2.5, 4, 5.5, 7
fig = go.Figure(go.Scatter(
    x=[0, 0, None, 0.03, 0.03, None, 0.03, 0.03, None, 0, 0, None],
    y=[low, q1, None, q1, median - 0.05, None,
       median + 0.05, q3, None, q3, high, None],
    mode="lines",
))
```

The five numbers above are an example, computed from the values 1 to 7 with linear quartiles.
Compute the five numbers for each distribution, and choose the offset and the gap at the median
for the scale of the data. Keep the box when its outliers, notches, hover information or
statistical convention are needed. The whiskers of a default box end at the last values within
1.5 times the interquartile range of the box, and the two ends need not be the minimum and the
maximum. A drawing from the minimum to the maximum therefore shows another statistic.

The book prefers the quartile plot with the middle half offset (page 124), and says that the
choice also rests on statistical and aesthetic criteria. The table gives what plotly 7.1.0 drew
for four groups. The numbers measure the drawing and do not show that the two forms are equal
as statistics, so the rule has no fix.

| Drawing of four groups | Straight segments | Filled box area in square pixels |
| --- | --- | --- |
| four default `go.Box` traces | 36 | 15,241.7 |
| a thinner box | 28, and the median was hidden | not recorded |
| the offset quartile plot in `go.Scatter` | 16 | 0 |
