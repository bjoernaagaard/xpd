On pandas 3, `"D"` means a calendar day, so shifting a time zone aware index by a day can raise at a daylight saving change. Write `"24h"` when you mean 24 hours.

```diff lang="python"
 ts = df.set_index("time").tz_localize("Europe/Copenhagen")
-day_before = ts.shift(freq="D")
+day_before = ts.shift(freq="24h")
```

## What the rule reports

The rule reports `ts.shift(freq="D")`, `ts.index + pd.offsets.Day(1)` and `ts.resample("D", offset="6h")` in a file that names a time zone. `"D"` was 24 hours on pandas 2 and is a calendar day on pandas 3. The table shows the result of each version for quarter-hourly data in `Europe/Copenhagen`.

| Code | pandas 2.3 | pandas 3.1 |
| --- | --- | --- |
| `ts.shift(freq="D")` | shifted by 24 hours | raised ValueError when the data held 31 March or 27 October 2024, because a shifted time fell in the daylight saving change |
| `ts.resample("D", offset="6h")` | started each bin at 06:00 | started each bin at 00:00 with only a RuntimeWarning, also on an index in UTC |

pandas 3.1 ignores `origin=` and a frequency of several days, such as `"7D"`, in the same way as `offset=`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `"24h"` for `"D"` and `"1D"` in `shift` and `pct_change`, and `pd.Timedelta(days=n)` for `pd.offsets.Day(n)`, which keeps 24 hours. Several days, `Day()` with no count, `resample` and `Grouper` have no fix.

`"24h"` gives the pandas 2 result on both versions. It reads the value 24 hours earlier, which is one hour off by the clock on the day after a daylight saving change. In `resample`, bins of `"24h"` keep the offset, and they start an hour later by the clock after a change to summer time.

xpd can't see whether an index has a time zone, so it reports only in a file that holds `tz_localize`, `tz_convert`, `tz=` or `utc=True`. It misses a frame that got its time zone in another file. On an index without a time zone both versions give the same result. In a file that names a time zone, the rule also reports a PeriodIndex of days by mistake, unless the name of the index holds the word period. On a PeriodIndex the fix `"24h"` raises.
