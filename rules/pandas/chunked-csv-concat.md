Reading a CSV in chunks and joining every chunk keeps the whole file in memory anyway, and took 3 times as long as one read. Drop `chunksize`, or reduce each chunk before you join them.

```diff lang="python"
-df = pd.concat(pd.read_csv(path, chunksize=100_000))
+df = pd.read_csv(path)
```

## What the rule reports

The rule reports `pd.concat(pd.read_csv(path, chunksize=n))`, also with `list(...)` or a comprehension around the reader. The code joins every chunk again, so the whole file is in memory anyway. The two frames were equal, and the table shows how long the chunked read took in two runs against one `read_csv` without `chunksize`.

| Threads of the reader | Time against one `read_csv` |
| --- | --- |
| the default, on a 32-core machine | 3.1 and 2.9 times as long |
| one | 1.1 and 1.0 times as long |

The rule leaves a loop alone that reduces each chunk before the concat.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.read_csv(path)` without `chunksize`, when concat takes nothing but the chunks and `ignore_index=True`. pandas guesses the dtypes per chunk, so a column with mixed values can differ. A concat with other arguments, such as `keys=`, has no fix.
