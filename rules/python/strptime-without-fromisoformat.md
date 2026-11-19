`strptime` with an ISO 8601 format does what `fromisoformat` does without a format. Call `date.fromisoformat` or `datetime.fromisoformat`.

```diff lang="python"
-from datetime import date, datetime
+from datetime import date
 
 
 def trading_day(text: str) -> date:
-    return datetime.strptime(text, "%Y-%m-%d").date()
+    return date.fromisoformat(text)
```

## What the rule reports

The rule reports `datetime.strptime` with a format that is ISO 8601: `"%Y-%m-%d"`, a date and
a time with `T` or a space between them, and either with `.%f` or `%z` at the end. It leaves
every other format alone, and a format that is held in a name.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`strptime` turns the format into a regular expression and matches the text against it.
[`date.fromisoformat`](https://docs.python.org/3/library/datetime.html#datetime.date.fromisoformat)
and `datetime.fromisoformat` read the text without a format. On Python 3.15.2 on one machine,
`datetime.strptime(text, "%Y-%m-%d").date()` took 1.53 microseconds and
`date.fromisoformat(text)` took 0.035. For `"%Y-%m-%dT%H:%M:%S"` the two took 1.86 and 0.044.

The two accept different text at the edges. `strptime` reads `2026-1-5`, and `fromisoformat`
raises a `ValueError` for a month or a day without its leading zero. `fromisoformat` also reads
other ISO 8601 spellings, such as `20261118`. Keep `strptime` for data that leaves out the
leading zeros.

ruff's `DTZ007` reports a `strptime` format without `%z`, and has no rule that asks for
`fromisoformat`.
