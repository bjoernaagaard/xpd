Returns given as decimals make the arch optimizer struggle, so it warns and can stop with poor estimates. Multiply the returns by 100, or pass `rescale=True`.

```diff lang="python"
-returns = prices.pct_change().dropna()
+returns = 100 * prices.pct_change().dropna()
 res = arch_model(returns).fit(disp="off")
```

## What the rule reports

The rule reports a series from `pct_change`, or from `log` with `diff` or `shift`, that is given to `arch_model`, `ConstantMean`, `ZeroMean`, `ARX`, `HARX`, `LS`, `MAMean` or `ARCHInMean` with nothing that multiplies it and no `rescale`.

## How to fix it

xpd has no automatic fix. The message names two rewrites, and the call does not show which one is meant. Make the change by hand.

The table gives a constant mean GARCH(1,1) fitted to the S&P 500 returns in four forms.

| Returns | arch | Warnings | Optimizer code | mu | omega |
| --- | --- | --- | --- | --- | --- |
| `100 * price.pct_change()` | 9.0.0, 8.0.0 | none | 0 | 0.0564 | 0.0175 |
| `price.pct_change()` | 9.0.0, 8.0.0 | DataScaleWarning, ConvergenceWarning | 4 | 4.02 | 0.0000029 |
| `np.log(price).diff()` | 9.0.0, 8.0.0 | DataScaleWarning, ConvergenceWarning | 4 | -2.73 | 0.0000029 |
| `price.pct_change()` | 5.0, 6.1.0, 6.2.0 | DataScaleWarning | 0 | -0.0011 | 0.0000029 |
| `price.pct_change()` with `rescale=True` | 9.0.0, 8.0.0 | none | 0 | 0.0564 | 0.0175 |

A mean of 4.02 for returns whose sample mean is 0.00021 is a failed fit, and on the older releases the
optimizer reported success with a mean of the wrong sign. `rescale=False` turns the warning off
and gave the same failed fit.

The rule reads how the series is built and not its values.

```text
report(series given to the model)
  if the series is written in the call, or every assignment in the file builds it
     as pct_change, the difference of a log, or the log of a price over its lag
     followed by nothing but dropna, fillna, copy, squeeze, to_numpy, iloc, loc or values
    report
leave alone
  a series that is multiplied anywhere       # 100 * prices.pct_change(), returns = returns * 100
  returns divided by their standard deviation, which have a variance of 1
  returns that a helper takes as an argument
  a call that says rescale, either way
not seen
  returns that come from a file or a function
```

All 21 reports on the public code were returns as decimals, and each series was a `pct_change`
or a log difference with no factor.
