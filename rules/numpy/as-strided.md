`as_strided` reads memory outside the array when a shape or a stride is wrong. Use `sliding_window_view` for windows, `broadcast_to` for repeated values, or slicing and `reshape` for a regular step.

```diff lang="python"
-windows = as_strided(x, shape=(len(x) - 2, 3), strides=(x.strides[0], x.strides[0]))
+windows = np.lib.stride_tricks.sliding_window_view(x, 3)
```

## What the rule reports

The rule reports `as_strided(...)`, because a wrong shape or stride reads memory outside the array. Its docs ask for extreme care and point to `sliding_window_view`, `broadcast_to` and `reshape`.

## How to fix it

xpd has no automatic fix. Because the shape and strides decide which safe function fits. Make the change by hand.
