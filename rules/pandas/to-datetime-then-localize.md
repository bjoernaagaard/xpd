`tz_localize("UTC")` after `to_datetime` raises on text that carries an offset. Pass `utc=True` to `to_datetime` instead.

```diff lang="python"
-when = pd.to_datetime(df["raw"]).dt.tz_localize("UTC")
+when = pd.to_datetime(df["raw"], utc=True)
```

## What the rule reports

The rule reports `pd.to_datetime(df["raw"]).dt.tz_localize("UTC")`, where `utc=True` sets the zone in the same call.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.to_datetime(df["raw"], utc=True)`.

The fix is unsafe, because on text that holds an offset `utc=True` converts to UTC where `tz_localize` raised TypeError. The fix adds an argument after the last one, so the rule leaves a call alone that ends in a trailing comma or holds a comment.
