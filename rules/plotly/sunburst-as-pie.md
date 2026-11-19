A sunburst draws each value as a wedge, and wedges in an outer ring get more area for each degree, so areas do not follow values. Show the hierarchy as a treemap or a sorted bar chart.

```diff lang="python"
-fig = px.sunburst(df, path=["region", "shop"], values="sales")
+fig = px.bar(df.sort_values("sales"), x="sales", y="shop", color="region", orientation="h")
```

## What the rule reports

The rule reports `px.sunburst(...)` with `values` or `path`, and `go.Sunburst(...)` with `values`.

The rule leaves alone a sunburst without numbers. A `go.Sunburst` with `labels` and `parents` and
no `values` gave each leaf one equal share, so it draws a hierarchy, such as a flavor wheel, and
not quantities. `px.sunburst` with `path` and no `values` counts the rows of each group, and the
rule reports it because the counts are numbers.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The book rejects pie charts for "their low data-density and failure to order numbers along a
visual dimension" (page 178). A sunburst draws each value as a wedge, as a pie does. Its rings
make the order worse, because the outer ring has three times the area of the inner ring for each
degree. The angles keep the order only within one ring. `px.sunburst` and
`go.Sunburst` resolved to a trace of type `sunburst`.

The test sunburst had the regions west 60 and east 40 in the inner ring, and the shops c 15 and
d 45 of west and a 30 and b 10 of east in the outer ring.

| Wedge | Value | Area in square pixels |
| --- | --- | --- |
| region west, inner ring | 60 | 7,963.9 |
| shop d, outer ring | 45 | 17,918.9, which is 2.25 times the area of west |

Shop a with 30 against region east with 40 was reversed in the same way.
