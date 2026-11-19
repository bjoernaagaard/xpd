bottleneck is not installed, so pandas computes `std`, `var`, `min`, `max` and `median` without it. Add bottleneck to the project, which made `std` 5.7 times faster on 10,000,000 rows.

```sh
uv add bottleneck
```

Add the package to the project environment, not to xpd itself.

## When xpd warns

pandas 3.1.1 uses bottleneck only for `std`, `var`, `min`, `max` and `median` with
`skipna=True` on a numeric column. The table shows each call on 10,000,000 float rows with
CPython 3.15.2, without bottleneck and with bottleneck 1.6.1.

| Call | Without | With | Times faster |
| --- | --- | --- | --- |
| `series.std()` | 49.20 ms | 8.61 ms | 5.71 |
| `series.var()` | 39.13 ms | 8.51 ms | 4.60 |
| `series.min()` | 16.37 ms | 3.52 ms | 4.65 |
| `series.max()` | 16.29 ms | 3.50 ms | 4.65 |
| `series.median()` | 98.74 ms | 77.56 ms | 1.27 |
| `series.agg("std")`, `frame.std()` | about 49 ms | about 8.7 ms | 5.7 |

The calls below gained nothing, because pandas does not hand them to bottleneck.

| Call | Without | With | Times faster |
| --- | --- | --- | --- |
| `series.mean()` | 16.37 ms | 16.70 ms | 0.98 |
| `series.sum()` | 14.59 ms | 14.69 ms | 0.99 |
| `series.std(skipna=False)` | 21.59 ms | 21.71 ms | 0.99 |
| `groupby.std()` | 25.07 ms | 24.54 ms | 1.02 |
| `groupby.median()` | 137.59 ms | 137.01 ms | 1.00 |
| `rolling.std()` | 141.04 ms | 141.62 ms | 1.00 |

The warning appears when bottleneck is not installed and a file that imports pandas calls a
method named `std`, `var`, `min`, `max` or `median`, or calls `agg` or `aggregate` with one of
those names as a string. A call with `skipna=False` does not count. A call whose receiver holds
`.groupby(`, `.rolling(`, `.resample(`, `.expanding(` or `.ewm(` does not count, and neither
does a call on `np`, `numpy`, `math`, `statistics` or `builtins`. The message gives the number
of such calls and the file and line of the first one.
