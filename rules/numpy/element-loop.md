A loop that computes one element of an array per pass runs Python once for every value, which was 70 to 530 times slower. Write it as one expression on whole arrays.

```diff lang="python"
-out = np.empty_like(a)
-for i in range(len(a)):
-    out[i] = a[i] * 2 + np.sqrt(b[i])
+out = a * 2 + np.sqrt(b)
```

## What the rule reports

The rule reports a `for` loop over `range`, `np.arange` or `np.ndindex`, and a `while i < n` loop with `i += 1`, when the loop's only statement writes one element of a NumPy array from arithmetic on the loop position. It also reports a loop over `np.nditer` or `np.ndenumerate` without `external_loop`. The rule leaves a loop alone whose body calls another function, reads the array it writes, or writes a slice.

## How to fix it

xpd has no automatic fix, so make the change by hand.
