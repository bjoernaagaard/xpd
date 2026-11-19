Parsing dates with `strptime` inside `apply` reads one text at a time, and was 24 times slower on a million rows. Parse the whole column with `pd.to_datetime` and a `format`.

```diff lang="python"
-when = df["raw"].apply(lambda text: datetime.strptime(text, "%Y-%m-%d"))
+when = pd.to_datetime(df["raw"], format="%Y-%m-%d")
```

## What the rule reports

The rule reports `df["raw"].apply(lambda v: datetime.strptime(v, "%Y-%m-%d"))`, and a lambda or `map` that calls `pd.to_datetime`, `pd.Timestamp` or `parser.parse` per value. On 1,000,000 rows `pd.to_datetime` with a format ran 24 times faster than `strptime`. The rule reports `strptime` only on `datetime`, because `time.strptime` returns another type.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.to_datetime(df["raw"], format="%Y-%m-%d")`.

The fix is unsafe, because a call per value parses each text on its own, while `pd.to_datetime` needs one format for the whole column. `strptime` also raises on a missing value, where `pd.to_datetime` returns NaT.
