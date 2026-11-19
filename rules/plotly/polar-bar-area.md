A polar bar draws each value as a wedge whose area grows with the square of the value. Draw the same values as a plain bar chart.

```diff lang="python"
-fig = px.bar_polar(wind, r="hours", theta="direction")
+fig = px.bar(wind, x="direction", y="hours")
```

## What the rule reports

The rule reports `px.bar_polar(...)` and `go.Barpolar(...)`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A polar bar is a wedge whose radius plotly scales with the value, so the area grows with the
square of the value. From 1 to 2 the data grows by 100 percent and the area by 300 percent, a
Lie Factor of 3.

| Value | Outer radius in pixels | Area ratio |
| --- | --- | --- |
| 1 | 42.75 | 1 |
| 2 | 85.5 | 4 |
| 3 | 128.25 | 9 |
| 4 | 171 | 16 |

The book adds that "the number of information-carrying (variable) dimensions depicted should not
exceed the number of dimensions in the data" (page 71), and a wedge varies in two dimensions to
show one number. The radial
axis takes the types linear, log, date and category, so no axis type draws the area in
proportion.
