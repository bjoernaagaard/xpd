This model has no analytic forecast beyond one step, so `forecast` with a longer horizon raises ValueError. Pass `method="simulation"` or `method="bootstrap"`.

```diff lang="python"
 res = arch_model(returns, vol="EGARCH", p=1, o=1, q=1).fit(disp="off")
-forecasts = res.forecast(horizon=5)
+forecasts = res.forecast(horizon=5, method="simulation")
```

## What the rule reports

The rule reports `forecast` with a `horizon` above 1 and the analytic method on a result of `arch_model` with `vol` `"egarch"`, `"aparch"` or `"fiaparch"`, or with a `power` that is not 2 and `vol` `"garch"`, `"figarch"` or no `vol`. Reports `hedgehog_plot` on such a result as well, unless the call says `horizon=1` or another method.

## How to fix it

xpd has no automatic fix. The message names two methods, and the call does not show which one is meant. Make the change by hand.

arch has no analytic formula for a forecast beyond one step when the volatility model is not a
model of the squared returns. `forecast(horizon=5)` was run for each `vol` on arch 9.0.0.

| Model | `forecast(horizon=5)` | `forecast(horizon=1)` |
| --- | --- | --- |
| `vol="egarch"`, `"aparch"`, `"fiaparch"` | ValueError: Analytic forecasts not available for horizon > 1 | runs |
| `vol="garch"` or `"figarch"` with `power=1.0` | ValueError: Analytic forecasts not available for horizon > 1 when power != 2 | runs |
| `vol="garch"`, also with `o=1`, `"arch"`, `"figarch"`, `"harch"`, `"ngarch"` | runs | runs |

The rule follows the model through the file, as in the call tree below. Each of the two names
must be assigned from nothing else.

```text
am = arch_model(returns, vol="EGARCH")     # such an arch_model call
  res = am.fit(...)                        # or am.fix(...)
    res.forecast(horizon=5)                # reported
```

A `horizon` or a `method` that is a variable is not read, and `power` is read only where
`arch_model` reads it. A model whose `volatility` the file assigns, as in
`am.volatility = GARCH(1, 0, 1)`, is left alone. A model that is built from the classes, such as
`ConstantMean(y, volatility=EGARCH())`, is not seen.

`hedgehog_plot` forecasts 10 steps with `method="analytic"` unless the call says otherwise. On
9.0.0 and on 8.0.0 it raised the same ValueError for `vol="egarch"`, and it ran with `horizon=1`
and with `method="simulation"`. The rule reads it only when every argument is a keyword, and
neither set of code had a report.
