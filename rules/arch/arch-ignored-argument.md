arch accepts this argument and never reads it for the model or test you asked for, so it has no effect. Pass the setting that does what you mean, such as `mean="AR"` for lags.

```diff lang="python"
-ar = arch_model(returns, lags=2)
-with_regressors = arch_model(returns, x=x, vol="garch", p=1, q=1)
-arch3 = arch_model(returns, vol="arch", q=3)
-adf = ADF(prices, lags=4, method="bic")
-paths = res.forecast(horizon=5, method="simulation", random_state=np.random.RandomState(0))
+ar = arch_model(returns, mean="AR", lags=2)
+with_regressors = arch_model(returns, x=x, mean="ARX", lags=0, vol="garch", p=1, q=1)
+arch3 = arch_model(returns, vol="arch", p=3)
+adf = ADF(prices, method="bic")
+paths = res.forecast(horizon=5, method="simulation", rng=np.random.default_rng(0).standard_normal)
```

## What the rule reports

The rule reports an argument that arch accepts and never reads for the model or the test that the call
asks for.

| Argument | Reported when the call has |
| --- | --- |
| `lags=` on `arch_model` | `mean` `"constant"`, `"zero"` or `"ls"`, or no `mean` |
| `x` on `arch_model` | `mean` `"constant"`, `"zero"`, `"ar"` or `"har"`, or no `mean` |
| `o=`, `q=` and `power=` | `vol="arch"` or `vol="harch"` |
| `power=` | `vol` `"egarch"`, `"aparch"`, `"fiaparch"` or `"ngarch"` |
| `o=` | `vol="figarch"` |
| `method=` and `max_lags=` on `ADF`, `DFGLS` or `ZivotAndrews` | a number for `lags` |
| `random_state=` on `forecast` | `method="simulation"` |
| `rng=` on `forecast` | `method="bootstrap"` |

The rule leaves alone a value that changes nothing in a GARCH model either, such as `o=0`, `q=0`
or `power=2.0`. It also leaves alone `lags=0`, `x=None`, and a `mean` or a `vol` that is a
variable. A call with three or more positional arguments is left alone, because its `mean` may
be positional.

## How to fix it

xpd has no automatic fix. Each message names two rewrites, and the call does not show which one is meant, and the `rng` form needs a seed that the file does not hold. Make the change by hand.

A reported argument does not change the fit. Each form was fitted with and without the argument
on 1,500 daily S&P 500 returns, on arch 9.0.0 and on 8.0.0, and the table gives whether the
parameters of the two fits were equal. `"fiaparch"` and `"ngarch"` are new in 9.0 and were run
there only.

| Call | Compared with | Parameters |
| --- | --- | --- |
| `arch_model(y, lags=2)`, and with `mean="zero"` or `"ls"` | the same call without `lags` | equal |
| `arch_model(y, mean="ar", lags=2)` | without `lags` | differ |
| `arch_model(y, x=x)`, and with `mean="zero"`, `"ar"` or `"har"` | without `x` | equal |
| `arch_model(y, x=x, mean="ls")`, and with `"arx"` or `"harx"` | without `x` | differ |
| `vol="arch"` or `"harch"` with `o=1`, `q=2` or `power=1.0` | `o=0`, `q=1`, `power=2.0` | equal |
| `vol="egarch"`, `"aparch"`, `"fiaparch"` or `"ngarch"` with `power=1.0` | `power=2.0` | equal |
| `vol="figarch"` with `o=1` | `o=0` | equal |
| `vol="garch"` with `o=1`, `q=2` or `power=1.0` | `o=0`, `q=1`, `power=2.0` | differ |

`ADF`, `DFGLS` and `ZivotAndrews` search for a lag length only when `lags` is not given. With
`lags=4` the statistic of 1,000 log prices was the same for `method` `"aic"`, `"bic"` and
`"t-stat"`, and the same for `max_lags` 2 and 20. Without `lags`, `"t-stat"` chose 8 lags where
`"aic"` chose 0, and the statistic of `ADF` moved from -1.2674 to -1.0838.

`forecast` reads `random_state` only for `method="bootstrap"` and `rng` only for
`method="simulation"`. The table gives the variance five steps ahead from two calls with the
same seed.

| Method | Seed | First call | Second call |
| --- | --- | --- | --- |
| `"simulation"` | `random_state=np.random.RandomState(0)` | 2.586 | 2.598 |
| `"simulation"` | a seeded `rng` | 2.571 | 2.571 |
| `"bootstrap"` | a seeded `rng` | 2.588 | 2.470 |
| `"bootstrap"` | `random_state=np.random.RandomState(0)` | 2.595 | 2.595 |

The page on volatility forecasts in arch's own documentation passes `random_state` to a
simulation. A call that passes both seeds is left alone.

The public code had 21 reports, and none had wrong advice.

| Reports | Code |
| --- | --- |
| 12 | `x=` or a positional `x` with a constant mean. Seven are copies of one function that fits `arch_model(resid, x=exog, mean="Constant", ...)` as if the regressors were in the model. |
| 4 | `vol="ARCH"` with `q=3` or `q=1`, where the order of an ARCH model is `p` |
| 2 | `random_state=` on a simulation |
| 2 | `lags=1` with `mean="constant"` |
| 1 | a `method` beside `lags=8` |

Of the 10 reports on arch's own code, one is a test that passes `max_lags=2.5` to check that it
raises, and its advice is wrong. The other nine are in tests and in the
documentation page on volatility forecasts.
