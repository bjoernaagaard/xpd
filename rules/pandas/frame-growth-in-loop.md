Adding one row at a time with `out.loc[len(out)] = row` copies the frame on each pass. Collect the rows in a list and build the frame once.

```diff lang="python"
-out = pd.DataFrame(columns=output_columns)
-for row in rows:
-    out.loc[len(out)] = row
+out = pd.DataFrame(rows, columns=output_columns)
```

## What the rule reports

The rule reports `out.loc[len(out)] = row` inside a loop, which grows the frame by one row at a time.

## How to fix it

xpd has no automatic fix, so make the change by hand.
