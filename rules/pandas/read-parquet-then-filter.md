Filtering rows right after `read_parquet` reads every row of the file first. Pass the test as `filters=`, so pyarrow skips the parts of the file that hold no matching row.

```diff lang="python"
-df = pd.read_parquet(path)
-df = df[df["id"] >= 900_000]
+df = pd.read_parquet(path, filters=[("id", ">=", 900_000)])
```

## What the rule reports

The rule reports `df = pd.read_parquet(path)` followed directly by `df = df[df["id"] >= value]`, with `>`, `>=`, `<`, `<=`, `==`, `!=` or `isin`. With `filters=`, pyarrow skips each row group that holds no matching row, and the values were equal. A row group is a block of rows that Parquet stores together. The rule has no fix, because the index can differ. A frame without a stored index came back numbered from 0, where the mask kept the old labels.

| File, filtered to the last tenth of a sorted column | Faster by |
| --- | --- |
| 10 row groups | 5.3 and 4.6 times in two runs, measured with the reader's default threads on a 32-core machine |
| 1 row group | 1.3 to 1.5 times |

## How to fix it

With `--unsafe-fixes`, xpd moves a literal integer lower bound into the Parquet read when the filter immediately follows a plain `read_parquet(path)` call. It selects PyArrow for row filtering.
