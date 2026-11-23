An encoder that adds random noise draws a new seed on every run, so the codes change from run to run. Pass `random_state`.

```diff lang="python"
-encoder = ce.LeaveOneOutEncoder(cols=["city"], sigma=0.05)
+encoder = ce.LeaveOneOutEncoder(cols=["city"], sigma=0.05, random_state=0)
```

## What the rule reports

The rule reports an encoder that adds random noise without `random_state`, and a `NestedCVWrapper`
that shuffles its folds without it.

## How to fix it

With `--unsafe-fixes`, xpd supplies `random_state=0`. Choose a different seed if your project uses one.

Two `fit_transform` calls on the same 400 rows gave codes that differed by the following largest
gaps:

| Form | Largest gap between two runs |
| --- | --- |
| `WOEEncoder(randomized=True)` | 0.15 |
| `JamesSteinEncoder(randomized=True)` | 0.13 |
| `MEstimateEncoder(randomized=True)` | 0.15 |
| `LeaveOneOutEncoder(sigma=0.05)` | 0.15 |
| `CatBoostEncoder(sigma=0.05)` | 0.15 |
| `NestedCVWrapper(TargetEncoder())` | 0.10 |

With `random_state=0` the two calls were equal in each row of the table, and a
`NestedCVWrapper` with `shuffle=False` was equal too. A seed does not move the expected score,
so the rule is info, as [`missing-random-state`](../sklearn/missing-random-state.md) of the scikit-learn
pack is. A `NestedCVWrapper` given a splitter as `cv` is left alone, because the splitter
brings its own seed. A file that calls `np.random.seed` is left alone, because the encoders then
draw from the global NumPy seed.
