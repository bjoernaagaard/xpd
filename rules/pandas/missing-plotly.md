The project has matplotlib and no plotly, so `df.plot` draws static matplotlib charts that xpd's plotly rules cannot check. Add plotly to the project and set it as the pandas plotting backend.

```sh
uv add plotly
```

Add the package to the project environment, not to xpd itself.

## When xpd warns

Without plotly, `df.plot` returns a static matplotlib Axes. With plotly as the plotting backend, `df.plot.bar(x="team", y="score")` returned the same figure as `px.bar(df, x="team", y="score")`, which xpd checks with its plotly rules.

The warning appears when plotly is not installed and the project uses matplotlib or seaborn.
