Grouping on dates formatted as text turns every date into text first. Group on `dt.to_period("M")`, `dt.floor("h")` or `pd.Grouper` instead.

```diff lang="python"
-monthly = df.groupby(df["time"].dt.strftime("%Y-%m"))["load"].sum()
+monthly = df.groupby(df["time"].dt.to_period("M"))["load"].sum()
```

## What the rule reports

The rule reports `df.groupby(df["d"].dt.strftime("%Y-%m"))`, which turns every date into text to build the group key.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The result has a PeriodIndex instead of text labels. On a column with a time zone, `to_period` drops the zone and gives a UserWarning, so group on `pd.Grouper(key="time", freq="MS")` instead.
