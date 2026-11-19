Drawing frame columns with matplotlib or seaborn gives a chart that xpd's plotly rules cannot check. Set the pandas plotting backend to plotly and draw with `df.plot`, which gave the same values as a plotly figure.

```diff lang="python"
-plt.scatter(df["score"], df["cost"])
+pd.options.plotting.backend = "plotly"
+
+df.plot.scatter(x="score", y="cost")
```

## What the rule reports

The rule reports `plt.plot`, `plt.scatter` and `plt.bar` on two columns of one frame, `plt.hist` on one column, and `sns.scatterplot`, `sns.histplot` and `sns.boxplot` with `data=`. The plot accessor drew the same values with the plotly backend. `plt` and `sns` must come from the import of `matplotlib.pyplot` and of `seaborn`.

| Left alone | Reason |
| --- | --- |
| `sns.barplot` and `sns.lineplot` | seaborn draws the mean of each group, and plotly draws every row |
| `plt.barh` | it takes the labels first, and with plotly `df.plot.barh` takes the values as `x` |

## How to fix it

xpd has no automatic fix, so make the change by hand.
