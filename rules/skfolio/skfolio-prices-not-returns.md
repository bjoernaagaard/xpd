The skfolio S&P 500, FTSE 100 and Nasdaq loaders return price levels. Optimizers expect period returns.

```diff
-EqualWeighted().fit(load_sp500_dataset())
+EqualWeighted().fit(prices_to_returns(load_sp500_dataset()))
```

## What the rule reports

The rule follows an unmodified loader result into fit, partial_fit or predict.
