Building a `Timedelta` for every value in Python was 1,549 times slower than converting the column. Call `pd.to_timedelta(s, unit="D")` on the whole column.

```diff lang="python"
-shift = df["days"].apply(lambda days: pd.Timedelta(days=days))
+shift = pd.to_timedelta(df["days"], unit="D")
```

## What the rule reports

The rule reports `df["n"].apply(lambda v: pd.Timedelta(days=v))`, the same with `timedelta` and `pd.DateOffset`, and `pd.to_timedelta(df["n"].astype(str) + " days")`. On 1,000,000 rows `pd.to_timedelta` ran 1,549 times faster. The rule leaves `pd.DateOffset(months=v)` alone, because a month has no fixed length.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.to_timedelta(df["n"], unit="D")` for `pd.Timedelta(v, unit="D")`, for a keyword such as `days=v`, and for the numbers written as text with a unit such as `" days"`. The Timedelta per value gave microseconds where `to_timedelta` gives nanoseconds, a DateOffset per value gave objects, and a number written with an exponent raised ValueError in the text. Any other text after the numbers has no fix.

`unit=` takes `"W"`, `"D"`, `"h"`, `"min"`, `"s"`, `"ms"` and `"us"`. On fractions the two forms can differ in the last digits.
