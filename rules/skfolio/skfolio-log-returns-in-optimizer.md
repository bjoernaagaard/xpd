skfolio portfolio weights combine asset returns linearly. Averaging asset log returns produces a different portfolio return.

```diff
-model.fit(prices_to_returns(prices, log_returns=True))
+model.fit(prices_to_returns(prices))
```

## What the rule reports

The rule follows a conversion with the explicit log option into a skfolio optimizer.

## How to fix it

Use `xpd check --fix --unsafe-fixes` when `prices_to_returns(..., log_returns=True)` is passed inline to switch `log_returns=False`.
