Calling `tz_localize` on a `date_range` builds the steps in local clock time and raises at a daylight saving change. Pass `tz=` to `date_range` instead.

```diff lang="python"
-hours = pd.date_range("2024-10-26", "2024-10-28", freq="h").tz_localize("Europe/Copenhagen")
+hours = pd.date_range("2024-10-26", "2024-10-28", freq="h", tz="Europe/Copenhagen")
```

## What the rule reports

The rule reports `pd.date_range(start, end, freq="h").tz_localize(zone)`, which builds the steps in local clock time and then fails on the daylight saving change.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.date_range(start, end, freq="h", tz=zone)`.

The fix is unsafe, because the two forms differ across a daylight saving change. Hourly steps from 26 to 28 October 2024 raised when localized, and gave 50 hours with `tz=`. Without a daylight saving change in the range, the two forms gave an equal index. The fix adds an argument after the last one, so the rule leaves a call alone that ends in a trailing comma or holds a comment.
