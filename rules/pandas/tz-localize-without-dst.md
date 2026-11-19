`tz_localize` raises on a time that repeats or is skipped at a daylight saving change unless you say what those times become. Pass both `ambiguous=` and `nonexistent=`.

```diff lang="python"
-local = df["time"].dt.tz_localize("Europe/Copenhagen")
+local = df["time"].dt.tz_localize("Europe/Copenhagen", ambiguous="infer", nonexistent="shift_forward")
```

## What the rule reports

The rule reports `df["d"].dt.tz_localize("Europe/Copenhagen")` without `ambiguous=` or without `nonexistent=`. A zone with daylight saving has an hour each year that happens twice and an hour that never happens. `tz_localize` raises on a time in either hour, unless `ambiguous=` and `nonexistent=` say what to do. On half-hourly data for 2024 in `Europe/Copenhagen` the call raised.

| Zone | Reported |
| --- | --- |
| a zone with daylight saving, such as `"Europe/Copenhagen"` | yes |
| `"UTC"`, `"GMT"`, a zone under `Etc/`, or an offset such as `"+01:00"` | no, because they did not raise |
| a zone held in a name | no |
| a zone without daylight saving, such as `"Asia/Tokyo"` | yes, by mistake |

## How to fix it

xpd has no automatic fix, so make the change by hand.

The table lists the values that the two arguments take.

| Argument | Value | Result |
| --- | --- | --- |
| `ambiguous=` | `"infer"` | reads the order of the rows, and needs both copies of the repeated hour |
| `ambiguous=` | `"NaT"` | makes the time missing |
| `ambiguous=` | a boolean array | says which rows are summer time |
| `nonexistent=` | `"shift_forward"`, `"shift_backward"`, `"NaT"` or a Timedelta | moves the time out of the missing hour, or makes it missing |
