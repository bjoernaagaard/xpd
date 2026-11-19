A gauge spends a whole dial on one number. Show the number with `mode="number+delta"`, or put a series of values in one small chart.

```diff lang="python"
-fig = go.Figure(go.Indicator(mode="gauge+number", value=270, gauge=dict(axis=dict(range=[0, 500]))))
+fig = go.Figure(go.Indicator(mode="number+delta", value=270, delta=dict(reference=250)))
```

## What the rule reports

The rule reports `go.Indicator` whose `mode` is a string that names `gauge`, such as `mode="gauge+number"`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

"Maximize data density and the size of the data matrix, within reason" (page 168). The book
calls a graphic that "purveys Graphical Style rather than quantitative information" a duck
(page 116), and a gauge draws a dial around one number.

| `go.Indicator` | Shapes drawn | Labels drawn |
| --- | --- | --- |
| `mode="gauge+number"` | 8 | 7 |
| `mode="number"`, the default | 0 | 2 |

plotly draws the dial only when `mode` names it. The rule does not report a `gauge` argument
with the default mode, because it drew no shape. A gauge with `shape="bullet"` is a bar and not a
dial, and is left alone. The rule has no fix, because a gauge can carry a threshold that the
number alone does not show.
