Filtering records in a Python comprehension before you build a frame tests every record in Python. Build the frame from all records, then select rows with a mask on the column.

```diff lang="python"
-frame = pd.DataFrame([row[1:] for row in rows if row[0] == "D"], columns=names)
+frame = pd.DataFrame(rows)
+frame = frame[frame[0] == "D"].drop(columns=0).set_axis(names, axis=1)
```

## What the rule reports

The rule reports `pd.DataFrame([... for row in rows if ...])`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The comprehension tests every record in Python before the frame exists. A mask on the column
compares all records at once. When the records come from a CSV file, `pd.read_csv` reads them
without a Python loop.
