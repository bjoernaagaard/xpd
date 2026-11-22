skfolio management fees use the period of each return observation. Transaction costs apply to differences from previous holdings.

```diff
-Portfolio(returns, weights, transaction_costs=0.01)
+Portfolio(returns, weights, previous_weights=starting_weights, transaction_costs=0.01)
```

## What the rule reports

An initial purchase can intentionally start with zero holdings. Supply previous weights for a rebalance and express transaction costs per traded amount. Convert annual management fees to the return period before passing them.
