`read_csv(path).head(n)` parses the whole file to keep a few rows. Pass `nrows=` to `read_csv`, which was 45 times faster.

```diff lang="python"
-sample = pd.read_csv(path).head(1000)
+sample = pd.read_csv(path, nrows=1000)
```

## What the rule reports

The rule reports `pd.read_csv(path).head(1000)` and `.iloc[:1000]`, which parse the whole file to keep the first rows. `nrows` was 45 times faster than `head`, measured with the reader's default threads on a 32-core machine.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `nrows=1000`, `nrows=5` for `head()` and `nrows=n` for `iloc[:n]` with a whole number. `iloc[:n]` with any other count has no fix.

The fix is unsafe, because pandas guesses the dtypes from the rows it reads, so a column can get another dtype.
