Since arch 9.0, these estimators use `opt_bandwidth` minus 1 as their bandwidth, so passing `opt_bandwidth` back gives another estimate. Pass the estimator's `bandwidth` instead.

```diff lang="python"
 first = Bartlett(returns)
-again = Bartlett(returns, bandwidth=first.opt_bandwidth)
+again = Bartlett(returns, bandwidth=first.bandwidth)
```

## What the rule reports

The rule reports `bandwidth=` set to the `opt_bandwidth` of an estimator, on `Bartlett`, `Gallant`, `NeweyWest`, `Parzen`, `ParzenCauchy`, `ParzenGeometric`, `ParzenRiesz`, `TukeyHamming`, `TukeyHanning` and `TukeyParzen`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `bandwidth=est.bandwidth` for `bandwidth=est.opt_bandwidth`. The fixed call repeats the first estimate, so the number changes, 0.5158 to 0.5245 in the Bartlett example.

A kernel covariance estimator reports the bandwidth it used as `bandwidth` and the optimal
bandwidth of Newey and West as `opt_bandwidth`. Before arch 9.0 the two were one number. Since
9.0 a kernel that is 0 beyond a finite lag uses `opt_bandwidth` minus 1, and the release notes
say that `Bartlett(x, bandwidth=est.opt_bandwidth)` is not the same as `Bartlett(x)`.

| Estimator on 1,500 S&P 500 returns | `bandwidth` | `opt_bandwidth` | Long run variance | With `bandwidth=opt_bandwidth` |
| --- | --- | --- | --- | --- |
| `Bartlett`, arch 9.0.0 | 10.943 | 11.943 | 0.52445 | 0.51582 |
| `Parzen`, arch 9.0.0 | 20.467 | 21.467 | 0.48023 | 0.47335 |
| `QuadraticSpectral`, arch 9.0.0 | 7.658 | 7.658 | 0.55401 | 0.55401 |
| `Bartlett`, arch 8.0.0 | 19.221 | 19.221 | 0.45172 | 0.45172 |

A script built every estimator class of `arch.covariance.kernel` on 9.0.0 and compared the two
attributes. The two attributes differ for the ten classes the rule names, and the rule leaves
alone `Andrews`, `QuadraticSpectral` and `ZeroLag`, where they are equal. The document carries
`since: "9.0"`, and neither set of code had a report. The automatic bandwidth itself also changed in 9.0,
from 19.221 to 10.943 for `Bartlett`, so an estimate without `bandwidth` differs between the
two releases.
