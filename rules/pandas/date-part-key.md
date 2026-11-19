Grouping on a year and a month uses two keys for one month and leaves out months with no rows. Group on `pd.Grouper(freq="MS")`, which gives one timestamp per month and was 6 times faster.

```diff lang="python"
-monthly = df.groupby([df["time"].dt.year, df["time"].dt.month])["load"].sum()
+monthly = df.groupby(pd.Grouper(key="time", freq="MS"))["load"].sum()
```

## What the rule reports

The rule reports `df.groupby([df["d"].dt.year, df["d"].dt.month])`, and the same with `index.year` and `index.month`. On 2,000,000 quarter-hourly rows in `Europe/Copenhagen` the two keys took 89 ms and one monthly key took 14 ms.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The result has one timestamp per month in place of a year and a month, and it also holds the months without rows.
