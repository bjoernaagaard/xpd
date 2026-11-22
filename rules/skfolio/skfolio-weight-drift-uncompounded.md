skfolio weight drift changes holdings and period returns. Compounding controls how those returns accumulate.

```diff
-Portfolio(returns, weights, weight_drift=True)
+Portfolio(returns, weights, weight_drift=True, compounded=True)
```

## What the rule reports

Two periods with returns of 10% and 10.9091% end with cumulative simple return of 0.2091 or wealth of 1.22 from starting wealth of 1. Set `compounded=True` separately on a `MultiPeriodPortfolio` when you want compounded wealth across child portfolios.
