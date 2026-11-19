`RandomRegressor` and `RandomAdder` without `random_state` draw new numbers on every call, so results change between calls. Pass `random_state=`.

```diff lang="python"
-baseline = RandomRegressor(strategy="normal")
+baseline = RandomRegressor(strategy="normal", random_state=0)
```

## What the rule reports

The rule reports `RandomRegressor` and `RandomAdder` built without `random_state`, outside test
files. Each call then draws new numbers.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Two `predict` calls of one fitted `RandomRegressor()` differed, and so did two `transform` calls of
one `RandomAdder()` on its training rows. Both pairs were equal with `random_state=0`.
