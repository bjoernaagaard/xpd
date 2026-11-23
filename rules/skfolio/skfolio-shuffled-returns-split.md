A shuffled split can train a skfolio model on returns from after the dates used for testing.

```diff
-train_test_split(returns)
+train_test_split(returns, shuffle=False)
```

## What the rule reports

The rule follows data from `prices_to_returns`. For cross-validation, use `WalkForward` to keep training before testing. KFold can train on future data even when its shuffle option is off.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to set `shuffle=False` on `train_test_split` calls over asset returns. Replace shuffled cross-validation splitters with `WalkForward` by hand.
