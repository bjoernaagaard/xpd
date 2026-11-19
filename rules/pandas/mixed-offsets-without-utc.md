On pandas 3, `to_datetime` raises ValueError when texts with a UTC offset hold more than one offset, as local times do across a daylight saving change. Pass `utc=True`, then convert with `dt.tz_convert`.

```diff lang="python"
-times = pd.to_datetime(df["time"], format="%Y-%m-%d %H:%M%z")
+times = pd.to_datetime(df["time"], format="%Y-%m-%d %H:%M%z", utc=True).dt.tz_convert("Europe/Copenhagen")
```

## What the rule reports

The rule reports `pd.to_datetime(s, format="%Y-%m-%d %H:%M%z")` without `utc=`, which raises ValueError on pandas 3.1 when the text holds more than one offset. Local times across a daylight saving change hold two offsets, such as `+0100` and `+0200`.

| Version, on text with offsets `+0100` and `+0200` | Result |
| --- | --- |
| pandas 2.3 | an object Index and a FutureWarning |
| pandas 3.1 | ValueError |

## How to fix it

xpd has no automatic fix, so make the change by hand.

xpd doesn't apply the replacement, because the zone that `dt.tz_convert` needs is not in the file. On a column that holds one offset, `utc=True` alone changes the result from the offset to UTC.
