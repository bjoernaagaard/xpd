`df.plot(kind="bar")` hides the kind in a text argument. Call the accessor method, as in `df.plot.bar(...)`, which draws the same figure.

```diff lang="python"
-df.plot(kind="bar", x="team", y="score")
+df.plot.bar(x="team", y="score")
```

## What the rule reports

The rule reports `df.plot(kind="bar", ...)` for the eleven kinds that the accessor has a method for: `line`, `bar`, `barh`, `box`, `hist`, `kde`, `density`, `area`, `pie`, `scatter` and `hexbin`. The two forms were equal as JSON with the plotly backend, and xpd reads the method form when pandas draws with plotly. The rule leaves alone a kind that only another backend knows, such as `"violin"` with plotly.

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `df.plot.bar(x="team", y="score")`.
