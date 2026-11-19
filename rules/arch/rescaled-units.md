`rescale=True` lets arch multiply the data, so the volatility, forecasts and parameters come back in rescaled units. Read `scale` and divide by it, or scale the data yourself.

```diff lang="python"
 res = arch_model(returns, rescale=True).fit(disp="off")
-variance = res.forecast(horizon=1).variance.iloc[-1, 0]
+variance = res.forecast(horizon=1).variance.iloc[-1, 0] / res.scale**2
```

## What the rule reports

The rule reports `rescale=True` on `arch_model` or a mean class in a file that reads `conditional_volatility` or calls `forecast` and never reads `scale`.

## How to fix it

xpd has no automatic fix. The division goes where the code reads each result, which the match does not reach. Make the change by hand.

With `rescale=True` arch may multiply the data before it estimates, and the result reports the
factor as `scale`. On the S&P 500 returns as decimals, arch 9.0.0 and 8.0.0 chose a scale of
100, and the parameters matched a fit of the returns in percent.

| Result | Rescaled units | Units of the data |
| --- | --- | --- |
| last conditional volatility | 1.9926 | 0.019926 |
| variance forecast one step ahead | 3.5965 | 0.00035965, which is 3.5965 divided by `scale` squared |

The rule is a warning, because it cannot see two cases where nothing is wrong. When the data
already has a variance near 1, the scale is 1.0 and nothing changes. Results that have no unit,
such as the standardized residuals, are right at any scale.

Of its 14 reports on the public code, 11 used a volatility or a forecast in rescaled units. Two
of the 11 divided by 10,000 in the code where the square of `scale` belongs. The other three
had wrong advice. Two fitted data that was made standard normal first, and one only divided the
residuals by the volatility. The 4 reports on arch's own code are tests of `rescale` itself, and
their advice is wrong as well.
