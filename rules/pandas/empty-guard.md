The mean, median and similar reductions of an empty Series are already NaN, with no error. Drop the test for an empty frame.

```diff lang="python"
-share = (capped["upper"] < LOW_PRICE).mean() if len(capped) else float("nan")
+share = (capped["upper"] < LOW_PRICE).mean()
```

## What the rule reports

The rule reports `x if len(frame) else nan` where `x` is `mean`, `median`, `std`, `var`, `min`,
`max` or `quantile` of a column picked with a string key. It leaves `sum` alone, because the sum
of an empty Series is 0, and it leaves a NumPy array alone, because `np.mean` of an empty array
warns.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes the reduction without the test, because the frame in the test and the frame in the reduction can differ.

xpd applies the replacement with `--fix --unsafe-fixes`.

With warnings turned into errors, `mean`, `median`, `std`, `var`, `min`, `max` and
`quantile(0.5)` of `pd.Series([], dtype=float)` each returned nan on pandas 3.1.1. The test
repeats what pandas does, and a reader has to check that both branches give the same answer.
