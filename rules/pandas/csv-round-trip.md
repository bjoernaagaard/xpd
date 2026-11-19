Writing a frame to CSV and reading it back makes pandas guess every dtype again, so dates, categories and nullable integers come back as other types. Use `to_parquet` and `read_parquet`, which kept the frame equal in a smaller file.

```diff lang="python"
-df.to_csv("clean.csv", index=False)
-again = pd.read_csv("clean.csv")
+df.to_parquet("clean.parquet")
+again = pd.read_parquet("clean.parquet")
```

## What the rule reports

The rule reports `df.to_csv(path)` in a file that also calls `pd.read_csv(path)` on the same path, written the same way in both calls. `read_csv` guesses every dtype again, and the table shows the dtypes that changed. `to_csv` took 0.93 seconds and `to_parquet` took 0.09 seconds.

| Column written | Read back from CSV |
| --- | --- |
| `datetime64[us]` | `str` |
| ordered category | `str` |
| `Int64` | `float64` |

The rule leaves a file alone that only writes a CSV, because another program may read it. It also leaves `to_csv` with `mode=` alone, which appends to a file.

## How to fix it

xpd has no automatic fix, so make the change by hand.
