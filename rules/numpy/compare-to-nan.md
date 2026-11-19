A comparison with NaN is False for every value, NaN included. Use `np.isnan` to find NaN.

```diff lang="python"
-missing = x == np.nan
+missing = np.isnan(x)
```

## What the rule reports

The rule reports `x == np.nan`, `x != np.nan`, `x == float("nan")`, `x == math.nan` and `np.nan in np.unique(x)`. None of them finds NaN, because NaN equals nothing.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.isnan(x)`, `np.isnan(np.unique(x)).any()` for `np.nan in np.unique(x)`, and `not np.isnan(np.unique(x)).any()` for `not in`. `isnan` raises on an array of text or objects, where `in` returned False.

In a file that imports pandas, the pandas rule [`compare-to-missing`](../pandas/compare-to-missing.md) reports the code instead. The rule leaves a comparison alone that is passed to an assert function, because a test may check how a comparison with NaN behaves.
