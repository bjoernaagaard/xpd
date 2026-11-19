`rolling(...).apply` with a lambda or a NumPy function runs Python once for every window, which was up to 1,031 times slower. Call the rolling method directly, as in `rolling(20).max()`.

```diff lang="python"
-average = df["load"].rolling(20).apply(lambda window: window.mean())
+average = df["load"].rolling(20).mean()
```

## What the rule reports

The rule reports `rolling(20).apply(lambda w: w.max())`, `rolling(20).apply(f)` without `raw=True`, `rolling(20).agg(np.max)` and `rolling(20).apply(np.mean, raw=True)`, which call Python once per window. On 200,000 rows `rolling(60).agg(np.max)` was 1,031 times slower than `rolling(60).max()`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `rolling(20).max()` for min, max and median. The unsafe fix writes the rolling method for sum, mean, std and var, and `rolling(20).max()` for a NumPy function such as `np.max` with sum, mean, min, max or median, which can differ in the last bit and skip a missing value. The `raw=True` case, which reports as info, and `np.std` and `np.var`, which divide by n where the window methods divide by n - 1, have no fix.

The fixes for sum, mean, std and var are unsafe, because the window methods keep running totals. Values far apart in size can then give another result. For a function with no window method, the message names `raw=True` and `engine="numba"`.
