pandas 3 stores datetimes in microseconds by default, so casting a datetime to an int and dividing by `10**9` is off by a factor of 1,000. Subtract the epoch and divide by a `pd.Timedelta`.

```diff lang="python"
-seconds = df["time"].astype("int64") // 10**9
+seconds = (df["time"] - pd.Timestamp("1970-01-01")) // pd.Timedelta("1s")
```

## What the rule reports

The rule reports `df["d"].astype("int64") // 10**9`, `index.asi8 // 10**9` and `df["d"].values.astype("float64") / 1e9`, which are off by a factor of 1,000 now that datetimes default to microseconds. It also reports `.view(np.int64)` divided the same way.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `(col - pd.Timestamp("1970-01-01")) // pd.Timedelta("1s")`, with `"1ms"` for `10**6` and `"1us"` for `10**3`. The result changes on purpose, a column with a time zone raises TypeError, because the epoch has none, a missing value gives NaN, and the fix of `asi8` and `.values` returns an Index or a Series where the old code returned a NumPy array.

The replacement assumes dates without a time zone, and for dates with a time zone you subtract `pd.Timestamp("1970-01-01", tz="UTC")` instead. A missing timestamp stays missing.
