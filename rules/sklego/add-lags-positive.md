`add_lags` with a positive lag copies the value of a later row, so lag 1 is the next row and leaks the future into the features. Use negative lags for features from the past.

```diff lang="python"
-features = add_lags(frame, "sales", [1, 2])
+features = add_lags(frame, "sales", [-1, -2])
```

## What the rule reports

The rule reports `add_lags` with a positive lag, as a number, in a list or from `range`.
`add_lags` shifts by the negative of the lag, so lag 1 puts the value of the next row on each row.
pandas `shift(1)` goes the other way.

## How to fix it

`xpd check --fix --unsafe-fixes` negates positive literal integer lags (single integer or list/tuple of positive integers). It is unsafe because a positive lag is intentional when creating a forecast target, and mixed lists or `range(...)` calls remain manual.

On a frame with sales 10 to 50, the two calls gave the following columns:

| Call | Values of the new column |
| --- | --- |
| `add_lags(frame, "sales", 1)` | 20, 30, 40, 50 |
| `add_lags(frame, "sales", -1)` | 10, 20, 30, 40, from the second row on |

On a series of pure noise, a model given the column from lag 1 predicted the sign of the next row
with accuracy 0.996. A model given the column from lag -1 predicted the sign of its own row with
accuracy 0.474. A positive lag is right when the new column is the target of a forecast, so the
rule is a warning.

A `range` is reported when its numbers are written out and it holds a positive lag, which means it
starts at 1 or more, or counts up to a stop of 2 or more. `range(0, -3, -1)` holds 0, -1 and -2
and is left alone.
