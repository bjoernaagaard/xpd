A 3D trace without a real third variable either draws nothing or adds a dimension the data does not have. Draw the variables in 2D.

```diff lang="python"
-fig = px.scatter_3d(df, x="area", y="rain")
-flat = go.Figure(go.Scatter3d(x=df["area"], y=df["rain"], z=np.zeros(len(df)), mode="markers"))
+fig = px.scatter(df, x="area", y="rain")
```

## What the rule reports

The rule reports `px.scatter_3d`, `px.line_3d` and `go.Scatter3d` with `x` and `y` and no `z`, and the same three with an `x`, `y` or `z` written as one constant: `[0] * n`, `np.zeros(n)`, `np.ones(n)`, `np.zeros_like(a)`, `np.ones_like(a)` or `np.full(n, v)`.

The rule leaves alone a call with a `**` argument, and a px call with `z` as its fourth
positional argument. It also leaves alone a file that sets `z` afterwards, as in
`trace.z = values` or `update_traces(z=values)`, because `go.Scatter3d` with `x` and `y` followed
by `trace.z = [3, 1, 2]` resolved to `visible=True`. The second form reads only a constant that
it can see in the call, with `np.` or `numpy.` in front of the function name.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is error for a 3D trace without `z`, and warning for a `z` that is one constant. The unsafe fix writes `px.scatter`, `px.line` or `go.Scatter` for the 3D function without `z`, and the same with `z` dropped when `z` is the constant and `x` and `y` vary. The 2D figure draws the points, and a `go.Scatter3d` line with a colorscale raises as a `go.Scatter`. A call with an argument that only a 3D trace takes, such as `range_z`, a constant `x` or `y`, or arguments by position has no fix.

"The number of information-carrying (variable) dimensions depicted should not exceed the
number of dimensions in the data" (page 71).

| Form | What plotly drew | Severity |
| --- | --- | --- |
| no `z` | nothing, because all three resolved the trace to `visible=False` in an empty scene | error |
| `z=[0, 0, 0]` | the points on one plane of a scene whose z axis ran from -1 to 1, so the third axis and the perspective show no data | warning |
