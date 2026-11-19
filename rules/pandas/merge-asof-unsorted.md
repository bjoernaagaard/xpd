`merge_asof` raises ValueError unless both frames are sorted by the key, and nothing here sorts them. Sort both frames by the key before the call.

```diff lang="python"
-matched = pd.merge_asof(trades, quotes, on="time")
+matched = pd.merge_asof(trades.sort_values("time"), quotes.sort_values("time"), on="time")
```

## What the rule reports

The rule reports `pd.merge_asof(...)` in a function that has no `sort_values` and no `sort_index`, and at module level in a module that has none. An unsorted left frame raised ValueError "left keys must be sorted", and an unsorted right frame raised "right keys must be sorted". The rule also reports a frame that another function sorts, by mistake.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.merge_asof(left.sort_values("t"), right.sort_values("t"), on="t")` when both frames are passed by position and the key by `on=`. The sort reorders the rows of the result. A frame passed by keyword, `left_on` and `right_on`, and the indexes have no fix.
