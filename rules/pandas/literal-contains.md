`str.contains` treats its pattern as a regular expression by default, even when the pattern is plain text. Pass `regex=False` for plain text, which is faster.

```diff lang="python"
-matches = df["name"].str.contains("load_")
+matches = df["name"].str.contains("load_", regex=False)
```

## What the rule reports

The rule reports `df["s"].str.contains("ta_")` with a pattern that has no regular expression character, where `regex=False` gives the same match faster.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `str.contains("ta_", regex=False)`. On a column of Python objects, a value that is not text gives NaN with a pattern and False with `regex=False`.
