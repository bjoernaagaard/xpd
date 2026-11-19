With a float step such as 0.1, the length of `np.arange` depends on rounding, and the stop value can appear in the result. Use `np.linspace` with the number of values and `endpoint=False`.

```diff lang="python"
-ticks = np.arange(0, 1, 0.1)
+ticks = np.linspace(0, 0.9, 10)
```

## What the rule reports

The rule reports `np.arange(0, 1, 0.1)`, whose length depends on rounding. For example, `np.arange(1, 1.3, 0.1)` ends with 1.3000000000000003. The [NumPy docs for arange](https://numpy.org/doc/stable/reference/generated/numpy.arange.html) recommend `np.linspace` for a step that is not whole.

The rule leaves a step alone that is a multiple of 1/64, such as 0.5 or 0.25, because such a step is exact.

## How to fix it

xpd has no automatic fix, so make the change by hand.
