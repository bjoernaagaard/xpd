A `GroupedArray` starts its first series at `indptr[0]`. An offset above zero leaves the preceding values outside any series.

```diff lang="python"
- groups = GroupedArray(values, np.array([3, 10]))
+ groups = GroupedArray(values, np.array([0, 3, 10]))
```

## What the rule reports

The rule reports a CoreForecast `GroupedArray` call whose second argument or `indptr=` is an inline `np.array` or `numpy.array` with a literal list or tuple starting with a positive decimal integer. It checks the constructor's import. Offsets computed elsewhere need a runtime check.

With CoreForecast 0.0.20 and ten values, `[3, 10]` defined one series with seven values. `[0, 3, 10]` defined the two intended series, with three and seven values. See the [grouped array contract](https://nixtlaverse.nixtla.io/coreforecast#minimal-example).

## How to fix it

`xpd check --fix --unsafe-fixes` prepends `0` to literal list or tuple offsets inside `np.array(...)` or `numpy.array(...)`. It is unsafe because you should also confirm that the last offset matches the length of the values array.
