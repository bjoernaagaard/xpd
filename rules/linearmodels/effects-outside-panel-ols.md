In linearmodels, only `PanelOLS` reads effect terms in a formula, and the other panel models drop them without an error. Use `PanelOLS` when you want fixed effects.

```diff lang="python"
-model = RandomEffects.from_formula("lwage ~ 1 + hours + EntityEffects", data)
+model = PanelOLS.from_formula("lwage ~ 1 + hours + EntityEffects", data)
```

## What the rule reports

The rule reports `EntityEffects`, `FixedEffects` or `TimeEffects` in a formula given to
`PooledOLS`, `RandomEffects`, `BetweenOLS`, `FirstDifferenceOLS` or `FamaMacBeth`. Every panel
model takes the three terms out of the formula, and only `PanelOLS` then uses them.

## How to fix it

xpd has no automatic fix, so make the change by hand.

On 7.1 and 7.0, each of the five classes returned the same coefficients for `"lwage ~ 1 + hours"` with and without each of the three terms, and raised no error and no warning. The reader of the formula sees fixed effects that the model does not have.
