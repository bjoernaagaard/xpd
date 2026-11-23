Calling a SciPy function that takes whole arrays once for every value was 7 to 23 times slower. Pass the whole array to the function.

```diff lang="python"
-p = np.array([stats.norm.cdf(v) for v in x])
+p = stats.norm.cdf(x)
```

## What the rule reports

The rule reports a comprehension or generator whose element is one SciPy call with the loop name
as an argument. A filter with `if`, a second `for`, and an element that does more than the call
are left alone.

| Element of the comprehension | One call on the array was faster by |
| --- | --- |
| a `scipy.special` ufunc | 7 to 23 times |
| a `scipy.stats` distribution method, such as `norm.cdf` | 92 to 730 times |
| a distribution frozen from the loop name, such as `norm(m, 2.0).pdf(x)` | 16 to 17 times |
| `stats.scoreatpercentile(x, p)` over 1,000 percentiles, which sorts the data on every call | 2.1 to 3.2 times |

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites single-argument `np.array([special.<ufunc>(x) for x in it])` and `np.array([stats.<dist>.<method>(x) for x in it])` comprehensions to a single array call with `np.asarray(it)`. Other comprehension and generator forms remain manual.
