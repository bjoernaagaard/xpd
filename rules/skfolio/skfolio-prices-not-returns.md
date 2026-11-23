The skfolio S&P 500, FTSE 100 and Nasdaq loaders return price levels. Optimizers expect period returns.

```diff
-EqualWeighted().fit(load_sp500_dataset())
+EqualWeighted().fit(prices_to_returns(load_sp500_dataset()))
```

## What the rule reports

The rule follows an unmodified loader result into fit, partial_fit or predict.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to wrap the price dataset call in `prices_to_returns(...)` when `prices_to_returns` is imported.
