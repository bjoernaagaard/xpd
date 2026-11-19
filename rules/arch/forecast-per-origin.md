Calling `forecast` once for each origin in a loop makes one call where one would do. Call it once with `start` set to the first origin, which returns a row for every origin.

```diff lang="python"
-rows = []
-for origin in range(first, len(returns)):
-    rows.append(res.forecast(horizon=10, start=origin).variance.iloc[0])
+variance = res.forecast(horizon=10, start=first).variance
```

## What the rule reports

The rule reports `forecast` with `start` set to the variable of a `for` loop or a comprehension over `range(...)`, when the loop calls neither `fit` nor `fix`.

The rule leaves alone a loop that fits inside, because the loop is then a rolling estimate where
each origin has its own parameters. It also leaves alone a loop over a few chosen origins, such
as a tuple. A call whose object or other arguments hold the loop variable is left alone too, as
in `fits[i].forecast(start=i)`, because each origin has its own result there. The one report is
a test in arch's own code that steps through the origins of a hedgehog plot.

## How to fix it

xpd has no automatic fix. The rewrite replaces the loop around the call. Make the change by hand.

`forecast(start=i)` returns one row for every origin, the row a forecast starts from, from `i`
to the end of the data. The loop computes the same rows many times over and keeps one. Both forms ran on the 5,030 S&P 500
returns with `horizon=10` and gave equal values.

| arch | Loop over the last 200 origins | One call |
| --- | --- | --- |
| 9.0.0 | 0.106 seconds | 0.0005 seconds |
| 8.0.0 | 0.329 seconds | 0.0056 seconds |
