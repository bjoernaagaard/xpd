SciPy marks `interp1d` and these other interpolation classes as legacy. Use the replacement that the message names, such as `make_interp_spline`.

```diff lang="python"
-tck = interpolate.splrep(x, y, s=0.1)
-smooth = interpolate.splev(grid, tck)
+spline = interpolate.make_splrep(x, y, s=0.1)
+smooth = spline(grid)
```

## What the rule reports

The rule reports the interpolation classes and functions that SciPy marks as legacy, and its
message names the replacement that SciPy recommends:

| Legacy name | Write instead |
| --- | --- |
| `interp1d` | `make_interp_spline(x, y, k=1)` for linear interpolation, with `axis=-1` when `y` has more than one dimension |
| `Rbf` | `RBFInterpolator`, which takes the points as one (n, d) array |
| `UnivariateSpline`, `splrep` | `make_splrep` |
| `InterpolatedUnivariateSpline` | `make_interp_spline` |
| `LSQUnivariateSpline` | `make_lsq_spline` |
| `splprep` | `make_splprep` |
| `splev` | a `BSpline` that you call, with `nu=n` where `splev` took `der=n` |
| `splint`, `splder`, `splantider` | the `integrate`, `derivative` and `antiderivative` methods of a `BSpline` |
| `spalde` | the derivative of a `BSpline` |
| `sproot` | `PPoly.from_spline(spl).roots()` |

The message for `interp1d` says what each function does outside the range of the data, as the
table shows. A form whose replacement was added in SciPy 1.7 or 1.15 is skipped on an older SciPy.

| Function | Outside the range of the data |
| --- | --- |
| `interp1d` | raises |
| `make_interp_spline` | extrapolates |
| `numpy.interp` | repeats the end value |

## How to fix it

xpd has no automatic fix. Because each replacement takes other arguments and returns a BSpline or another object. Make the change by hand.
