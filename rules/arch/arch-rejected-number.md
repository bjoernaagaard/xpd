arch refuses this number, such as a float where it counts something, so the call raises. Write a value that arch takes, such as the number without its decimal point.

```diff lang="python"
-test = ADF(prices, lags=5.0)
-model = arch_model(returns, vol="ngarch")
-interval = IIDBootstrap(returns, seed=0).conf_int(np.mean, reps=1000, size=95)
+test = ADF(prices, lags=5)
+model = arch_model(returns, vol="ngarch", o=1)
+interval = IIDBootstrap(returns, seed=0).conf_int(np.mean, reps=1000, size=0.95)
```

## What the rule reports

The rule reports a number written in the call that arch refuses: a float where arch counts something, a `size` of `conf_int` of 1 or more, and `vol="ngarch"` with `o` left at 0.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The fix is unsafe even though the old call raises. A float with nothing but zeros after the point loses the point, as in `p=1` for `p=1.0`, `size=` of two digits becomes the fraction, as in `size=0.95` for `size=95`, and `o=1` is added after `vol="ngarch"` when the call has no `o`. A float such as `2.5`, any other `size`, and an `o=0` that is written have no fix.

Each form was run on arch 9.0.0 and on 8.0.0.

| Form | arch 9.0.0 | arch 8.0.0 |
| --- | --- | --- |
| `p=1.0`, `o=1.0` or `q=1.0` in `GARCH`, `EGARCH` or `APARCH`, and in `arch_model` with no `vol` or with `vol` `"garch"`, `"egarch"`, `"aparch"`, `"fiaparch"` or `"ngarch"` | TypeError: p must be an integer >= 0; got 1.0 (float) | the classes ran, and `arch_model(p=1.0)` raised AssertionError |
| `p=1.0` in `ARCH` or `FIGARCH` | TypeError: p must be an integer >= 1; got 1.0 (float) | ran |
| `lags=5.0` in `ADF`, `DFGLS`, `KPSS`, `PhillipsPerron`, `ZivotAndrews`, `VarianceRatio` or `ARX` | TypeError: lags must be an integer >= 0 or None; got 5.0 (float) | ran, apart from `PhillipsPerron` |
| `max_lags=5.0` in `ADF`, `DFGLS` or `ZivotAndrews` | TypeError: max_lags must be an integer >= 0 or None | TypeError: expected a sequence of integers or a single integer |
| `reps=10.0` in `conf_int`, `var`, `cov`, `apply`, `bootstrap`, `SPA`, `StepM` or `MCS` | TypeError: reps must be an integer >= 1; got 10.0 (float) | TypeError: 'float' object cannot be interpreted as an integer |
| `forecast(horizon=2.0)` and `hedgehog_plot(horizon=2.0)` | TypeError: horizon must be an integer >= 1; got 2.0 (float) | ValueError: horizon must be an integer >= 1. |
| `forecast(..., method="simulation", simulations=100.0)` | TypeError: simulations must be an integer >= 1 | TypeError: 'float' object cannot be interpreted as an integer |
| `conf_int(size=95)`, `size=90.0`, `size=1` | ValueError: size must be a finite float in (0, 1); got 95 (int) | ValueError: size must be strictly between 0 and 1 |
| `arch_model(vol="ngarch")` with no `o` or with `o=0` | ValueError: o must be at least 1 when vol='ngarch' | ValueError: Unknown model type in vol, for every `o` |

The documents of the first three rows and of the last carry `since: "9.0"`, because 8.0.0 took
the float in the classes and has no `"ngarch"`. `arch_model(vol="ngarch", o=1)` ran on 9.0.0,
and `o=2` raised because `o` may not be above `p`.

`arch_model` does not check an order that the process does not have, so the rule reads
`arch_model` only with the `vol` values of the first row. For example, `o=1.0` and `q=1.0` ran
with `vol="arch"` and `"harch"`, `o=1.0` ran with `"figarch"`, and all three ran with
`"constant"`.

| Keyword | Where the rule reads it | Why |
| --- | --- | --- |
| `horizon`, `simulations` | only on a result that the file gets from `arch_model` | other libraries have a `forecast` with a horizon |
| `reps`, `size` | by the name of the method | no other library that was looked at has the keywords on `conf_int`, `var`, `cov`, `apply` or `bootstrap` |

A number that is a variable is not read, and neither is a positional argument. The rule was
added after a review and was not run over the public code, which was no longer on disk. Over the
189 Python files of arch's repository it made one report, on `conf_int(mean_fn, 20, size=1.5)`
in a test that checks the error.
