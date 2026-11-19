Building a new frame from three or more columns of another frame copies them one at a time and is easy to get out of step. Rename and select the columns instead, and convert the ones that change with `assign` or in the call that reads the file.

```diff lang="python"
-lots = pd.DataFrame(
-    {
-        "lot": raw["Auction Lot ID"],
-        "side": raw["Buy Sell"].str.lower(),
-        "cleared_mw": raw["Cleared Volume"],
-    }
-)
+NAMES = {"Auction Lot ID": "lot", "Buy Sell": "side", "Cleared Volume": "cleared_mw"}
+
+lots = raw.rename(columns=NAMES)[list(NAMES.values())]
+lots = lots.assign(side=lots["side"].str.lower())
```

## What the rule reports

The rule reports `pd.DataFrame({...})` with a dict in which three or more values are taken from
columns of the same frame.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The dict copies the frame one column at a time, and the old name, the new name and the
conversion of each column are mixed in one long literal. The mapping states the names once and
also selects the columns, and `assign` shows the few columns that change. On pandas 3.1.1 the
two forms gave equal frames. When the frame comes from `read_csv`, give the conversions to that
call with `usecols=`, `dtype=` and `parse_dates=`.
