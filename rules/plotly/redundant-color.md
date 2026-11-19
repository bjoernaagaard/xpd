Coloring by the same column that an axis already shows encodes one variable twice and adds a legend for nothing. Drop `color`, or color by another column.

```diff lang="python"
-fig = px.bar(df, x="day", y="tip", color="day")
+fig = px.bar(df, x="day", y="tip")
```

## What the rule reports

The rule reports a px call whose `color` is the same column name as its `x` or its `y`, both written as strings. It covers the 13 px functions that take `color`, `x` and `y`: `px.area`, `px.bar`, `px.box`, `px.density_contour`, `px.ecdf`, `px.funnel`, `px.histogram`, `px.line`, `px.line_3d`, `px.scatter`, `px.scatter_3d`, `px.strip` and `px.violin`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

unsafe for px: drops `color`, so the chart draws one color and no legend, and drops comments inside the parentheses. None for `go.Bar`, where the one color to use is not in the code.

"Erase redundant data-ink, within reason" (page 100). `px.bar(x="day", y="tip")` drew one trace
and no legend. With `color="day"` it drew four traces and a legend with the entries Fri, Sat,
Sun and Thur, the same four words as the tick labels under the bars. With `color="tip"` it
drew a colorbar for the numbers that the heights of the bars already show.

One color for each category is sometimes meant, to match the colors of another chart of the
same categories, which is why the rule is info. A column given by a name, as in
`color=column`, is left alone.

The rule also reads `go.Bar` whose `marker_color`, or `color` inside `marker`, is the same
expression as its `x` or `y`, as in `go.Bar(x=names, y=df["sales"], marker_color=df["sales"])`.
The length of a bar already shows its value, so the colors repeat it.

| `go.Bar(y=[1, 2, 4], ...)` | Colors of the three bars |
| --- | --- |
| without `marker_color` | `rgb(99, 110, 250)` for all three |
| `marker_color=[1, 2, 4]` | `rgb(13, 8, 135)`, `rgb(156, 23, 158)` and `rgb(240, 249, 33)` |

A written string or number as the color is left alone. When the repeated coordinate holds
categories, the colors repeat the categories and not the lengths of the bars. For example,
`go.Bar(x=["red", "blue"], y=[1, 2], marker_color=["red", "blue"])` gives each category a color.
Keep such colors when they match another chart or serve another purpose.
