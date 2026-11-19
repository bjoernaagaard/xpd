A list comprehension or a loop over a column runs Python once for every value, which was 13 to 19 times slower. Use a column method such as `.str.upper()` instead.

```diff lang="python"
-names = [name.upper() for name in df["name"]]
+names = df["name"].str.upper()
```

## What the rule reports

The rule reports `[v.upper() for v in df["s"]]`, and a `for` loop over a column that appends to a list, which was 13 to 19 times slower than the column method. The rule reports only a value before `[` that is named like a frame, which means `df`, `frame`, or a name with one of them as a word. It leaves `x["key"]` in a `for` clause alone, because such a value is more often a list inside a dict. The rule doesn't run on test files.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The result is a Series with row labels instead of a Python list. Call `tolist` only where the next step needs a list.
