Passing a math function such as `np.sqrt` to `apply` or `map` calls it once for every value. Call the function on the whole column.

```diff lang="python"
-root = df["wind"].map(np.sqrt)
+root = np.sqrt(df["wind"])
```

## What the rule reports

The rule reports `df["w"].map(np.sqrt)`, `df["w"].apply(lambda v: np.log(v))`, `df["w"].apply(math.sqrt)` and `df["b"].apply(abs)`, which call the function once per value. With a bare function name it reports `apply` only on a column taken by a text label, because on a frame `apply` works per column. It leaves `df["w"].apply(np.sqrt)` alone, because `Series.apply` already hands a NumPy function the whole column. `map` calls the function once per value.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.sqrt(df["w"])`, `df["b"].abs()`.

The fix is unsafe, because `math.sqrt` and `math.log` raise on a value outside their domain, where NumPy returns NaN.
